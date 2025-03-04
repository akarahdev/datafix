mod builtins;
mod dynamic;
mod ops;

use alloc::{boxed::Box, rc::Rc, string::String, sync::Arc, vec::Vec};
use builtins::{
    codecs::{
        ArcCodec, BoundedCodec, BoxCodec, DispatchCodec, DynamicCodec, EitherCodec, FnCodec,
        ListCodec, OrElseCodec, PairCodec, TryElseCodec, XMapCodec,
    },
    records::{OptionalField, RecordField, UnitCodec},
};
use core::{cell::RefCell, fmt::Debug, marker::PhantomData, ops::RangeBounds};
use either::Either;

pub use dynamic::*;
pub use ops::*;

use crate::result::DataResult;
pub use builtins::record_builder::MapCodecBuilder;

/// A [`Codec<T>`] describes transformations to and from [`Dynamic`] for a type `T`.
/// [`Codec`]s are lazy, they don't do anything by themselves.
/// You need to call [`Codec::encode`], [`Codec::decode`] to change between `T` and [`Dynamic`].
/// For more complex use cases, you can call helper methods on [`CodecAdapter`] such as [`CodecAdapter::list_of`] and [`CodecAdapter::xmap`].
///
/// For implementors, try to keep implementations of this trait pure, immutable, and deterministic.
///
/// [`Dynamic`]: [`dynamic::Dynamic`]
/// [`CodecAdapter`]: [`serialization::CodecAdapter`]
/// [`CodecAdapter::xmap`]: [`serialization::CodecAdapter::xmap`]
/// [`CodecAdapter::list_of`]: [`serialization::CodecAdapter::list_of`]
#[diagnostic::on_unimplemented(
    message = "{Self} is not a codec",
    label = "{Self} must be a Codec<{Type}>",
    note = "some types provide an implementation of DefaultCodec::codec()"
)]
pub trait Codec<Type, OpsType: Clone, Ops: CodecOps<OpsType>> {
    /// Transform a value of type `T` into a `U` using the provided [`CodecOps`], optionally returning an error .
    /// For implementors, this function should be pure and have no side effects.
    fn encode(&self, ops: &Ops, value: &Type) -> DataResult<OpsType>;
    /// Transforms a `U` value into a type `T` using the provided [`CodecOps`], optionally returning an error.
    /// For implementors, this function should be pure and have no side effects.
    fn decode(&self, ops: &Ops, value: &OpsType) -> DataResult<Type>;
}

/// Holds the adapter functions for [`Codec`] to allow codecs to do things such as:
/// - Turn into record fields
/// - Convert between types
/// - Have a list codec that contains the type of the provided codec
pub trait CodecAdapters<T, OT: Clone, O: CodecOps<OT>>
where
    Self: Sized + Codec<T, OT, O>,
{
    /// Returns a codec of this type that is intended for a field of a record.
    fn field_of<Struct>(
        self,
        name: impl Into<String>,
        getter: fn(&Struct) -> &T,
    ) -> RecordField<T, Self, Struct, OT, O> {
        RecordField {
            field_name: name.into(),
            getter,
            codec: self,
            _phantom: PhantomData,
        }
    }

    /// Returns a codec of an [`Option`] wrapping this type, that is intended for an optional field of a record.
    fn optional_field_of<Struct>(
        self,
        name: impl Into<String>,
        getter: fn(&Struct) -> &Option<T>,
    ) -> OptionalField<T, Self, Struct, OT, O> {
        OptionalField {
            field_name: name.into(),
            getter,
            codec: self,
            _phantom: PhantomData,
        }
    }

    /// Returns a codec that is a list of this codec.
    fn list_of(self) -> impl Codec<Vec<T>, OT, O> {
        ListCodec {
            inner: self,
            _phantom: PhantomData,
        }
    }

    /// Maps the output of this codec between 2 transformation functions.
    /// Implementors should hold the invariant of `F(G(x)) = x` such that the functions can be used to freely convert between the two types.
    fn xmap<U, F, G>(self, to_new: F, from_new: G) -> impl Codec<U, OT, O>
    where
        F: Fn(&T) -> U,
        G: Fn(&U) -> T,
    {
        XMapCodec {
            inner: self,
            f1: to_new,
            f2: from_new,
            _phantom: PhantomData,
        }
    }

    /// This returns a Codec that represents a pair of two codecs.
    fn pair<R>(self, right: impl Codec<R, OT, O>) -> impl Codec<(T, R), OT, O> {
        PairCodec {
            left: self,
            right,
            _phantom: PhantomData,
        }
    }

    /// This bounds the result of this codec in the range, returning an error if the value is not within the range.
    fn bounded(self, range: impl RangeBounds<T>) -> impl Codec<T, OT, O>
    where
        T: PartialOrd + Debug,
    {
        BoundedCodec {
            codec: self,
            range,
            _phantom: PhantomData,
        }
    }

    /// If this codec fails to encode or decode, it will fall back to using the second codec, only failing if both this and
    /// the other codec fail.
    fn try_else(self, other: impl Codec<T, OT, O>) -> impl Codec<T, OT, O> {
        TryElseCodec {
            lc: self,
            rc: other,
            _phantom: PhantomData,
        }
    }

    /// If decoding for this codec fails, provide a default value that will be used instead.
    /// If you are trying to make an optional field in a map, use [`CodecAdapters::optional_field_of`] instead.
    fn or_else<F: Fn() -> T>(self, f: F) -> impl Codec<T, OT, O> {
        OrElseCodec {
            codec: self,
            default: f,
            _phantom: PhantomData,
        }
    }

    /// Wraps this codec in a `Box<dyn Codec<...>>`, allowing it to be used in dynamic contexts where you
    /// only know which codec will be passed in at runtime. This also creates a pointer to a codec,
    /// enabling self-referential codecs.
    fn dynamic(self) -> DynamicCodec<T, OT, O>
    where
        Self: 'static,
    {
        DynamicCodec {
            codec: Box::new(self),
        }
    }

    /// Wraps this codec in an `Arc`, allowing it to be cloned and shared across threads.
    fn arc(self) -> ArcCodec<T, OT, O>
    where
        Self: 'static,
    {
        ArcCodec {
            codec: Arc::new(self),
        }
    }

    /// Wraps the value being serialized or deserialized in a [`Box`].
    fn boxed(self) -> BoxCodec<T, OT, O, Self> {
        BoxCodec {
            inner: self,
            _phantom: PhantomData,
        }
    }
}

impl<T, OT: Clone, O: CodecOps<OT>, C: Codec<T, OT, O>> CodecAdapters<T, OT, O> for C {}

/// This trait is the go-to trait for when you want to provide a [`Codec`] for a type. These should be used whenever possible.
/// Please keep try to keep your implementations const-safe as this function in a future version of Rust may be upgraded to a `const fn`.
pub trait DefaultCodec<OT: Clone, O: CodecOps<OT>>
where
    Self: Sized,
{
    /// Returns the default codec for a type.
    fn codec() -> impl Codec<Self, OT, O>;
}

/// This type provides associated methods for creating a variety of types of codecs.
pub struct Codecs;

impl Codecs {
    /// Creates a [`Codec`] you can use for serializing and deserializing recursive types.
    ///
    /// For example, if you wanted to create a recursive codec for a linked list, you coudl do so with such:
    /// ```rs
    /// #[derive(Clone, PartialEq, Debug)]
    /// struct LinkedList {
    ///     value: i32,
    ///     next: Option<Box<LinkedList>>,
    /// }
    ///
    /// Codecs::recursive(|codec| {
    ///     MapCodecBuilder::new()
    ///         .field(i32::codec().field_of("value", LinkedList::value))
    ///         .field(codec.boxed().optional_field_of("next", LinkedList::next))
    ///         .build(LinkedList::new)
    /// });
    /// ```
    pub fn recursive<
        T: 'static,
        OT: Clone + 'static,
        O: CodecOps<OT> + 'static,
        F: Fn(DynamicCodec<T, OT, O>) -> Oc,
        Oc: Codec<T, OT, O> + 'static,
    >(
        f: F,
    ) -> ArcCodec<T, OT, O> {
        let placeholder = Rc::new(RefCell::new(None::<ArcCodec<_, _, _>>));
        let placeholder_clone_1 = placeholder.clone();
        let placeholder_clone_2 = placeholder.clone();

        let dummy = DynamicCodec {
            codec: Box::new(FnCodec {
                encode: Box::new(move |ops, value| {
                    placeholder_clone_1
                        .borrow()
                        .as_ref()
                        .expect("tried to decode before initialization")
                        .encode(ops, value)
                }),
                decode: Box::new(move |ops, value| {
                    placeholder_clone_2
                        .borrow()
                        .as_ref()
                        .expect("tried to decode before initialization")
                        .decode(ops, value)
                }),
            }),
        };

        let codec = f(dummy).arc();

        *placeholder.borrow_mut() = Some(codec.clone());

        codec
    }

    pub fn either<
        T: 'static,
        T2: 'static,
        OT: Clone + 'static,
        O: CodecOps<OT> + 'static,
        Lc: Codec<T, OT, O> + 'static,
        Rc: Codec<T2, OT, O> + 'static,
    >(
        left: Lc,
        right: Rc,
    ) -> impl Codec<Either<T, T2>, OT, O> {
        EitherCodec {
            lc: left,
            rc: right,
            _phantom: PhantomData,
        }
    }

    pub fn dispatch<
        T,
        OT: Clone,
        O: CodecOps<OT>,
        E: Fn(&T) -> DataResult<DynamicCodec<T, OT, O>>,
        F: Fn(&O, &OT) -> DataResult<DynamicCodec<T, OT, O>>,
    >(
        from_type_to_codec: E,
        from_ops_to_codec: F,
    ) -> impl Codec<T, OT, O> {
        DispatchCodec {
            from_ops_to_codec,
            from_type_to_codec,
            _phantom: PhantomData,
        }
    }

    pub fn unit<OT: Clone, O: CodecOps<OT>>() -> impl Codec<(), OT, O> {
        UnitCodec {}
    }
}
