pub trait SchemaInitializer: Send + Sync {
    fn init_schema(&self) -> anyhow::Result<()>;
}
