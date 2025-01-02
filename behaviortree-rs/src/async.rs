use typed_builder::TypedBuilder;

#[derive(Debug, Default)]
pub enum RuntimeImplementation {
    #[default]
    None,
    #[cfg(feature = "async-tokio")]
    Tokio,
}

#[derive(Debug, Default)]
pub(crate) enum RuntimeHandle {
    #[default]
    Uninitialized,
    #[cfg(feature = "async-tokio")]
    InternalTokio(tokio::runtime::Runtime),
    #[cfg(feature = "async-tokio")]
    ExternalTokio(tokio::runtime::Handle),
}

impl RuntimeHandle {
    #[cfg(feature = "async-tokio")]
    /// Pass in a runtime handle from an external Tokio runtime
    pub fn external_tokio(handle: tokio::runtime::Handle) -> Self {
        Self::ExternalTokio(value)
    }

    #[cfg(feature = "async-tokio")]
    /// Create a new single-threaded runtime and let the `Tree` handle the runtime. You should only use
    /// this if you are running it from a non-async running environment and
    /// you only run a single behavior tree. Otherwise, it would be much more
    /// efficient to create your own runtime externally and own it in your code.
    pub fn tokio() -> Result<Self, tokio::io::Error> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        
        Ok(Self::InternalTokio(rt))
    }
}

#[derive(Default, TypedBuilder)]
pub struct AsyncRuntime {
    variant: RuntimeImplementation,
    handle: RuntimeHandle,
}
