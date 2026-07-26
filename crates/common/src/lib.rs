use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init_tracing(env: &str) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(if env == "development" {
            "lotus_connect_system=debug,gateway=debug,info"
        } else {
            "lotus_connect_system=info,gateway=info,warn"
        })
    });

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Tracing initialized for environment: {}", env);
}
