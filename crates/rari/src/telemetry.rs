use std::{borrow::Cow, collections::HashSet, env, sync::OnceLock};

use deno_telemetry::{OtelConfig, OtelConsoleConfig, OtelPropagators, OtelRuntimeConfig};
use sys_traits::impls::RealSys;

const CONSOLE_ENV: &str = "RARI_OTEL_CONSOLE";

static CONFIG: OnceLock<OtelConfig> = OnceLock::new();

pub fn init() {
    let Some(config) = config_from_env(|key| env::var(key).ok()) else {
        return;
    };

    let runtime = OtelRuntimeConfig {
        runtime_name: Cow::Borrowed("rari"),
        runtime_version: Cow::Borrowed(env!("CARGO_PKG_VERSION")),
    };
    if let Err(error) = deno_telemetry::init(&RealSys, runtime, config.clone()) {
        tracing::warn!("OpenTelemetry is disabled: {error}");
        return;
    }

    tracing::info!("OpenTelemetry export enabled");
    let _ = CONFIG.set(config);
}

pub fn shutdown() {
    if CONFIG.get().is_some() {
        deno_signals::run_exit();
    }
}

pub fn bootstrap_script() -> Option<String> {
    let config = CONFIG.get()?;
    Some(format!(
        "(() => {{
    const {{ core }} = globalThis.Deno;
    core.loadExtScript('ext:deno_telemetry/util.ts');
    core.loadExtScript('ext:deno_telemetry/telemetry.ts').bootstrap({:?});
}})();",
        config.as_v8()
    ))
}

fn config_from_env(var: impl Fn(&str) -> Option<String>) -> Option<OtelConfig> {
    let is_set = |key: &str| var(key).is_some_and(|value| !value.trim().is_empty());

    if var("OTEL_SDK_DISABLED").is_some_and(|value| value.trim().eq_ignore_ascii_case("true")) {
        return None;
    }
    let has_exporter = is_set("OTEL_EXPORTER_OTLP_ENDPOINT")
        || is_set("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT")
        || var("OTEL_EXPORTER_OTLP_PROTOCOL").is_some_and(|value| value.trim() == "console");
    if !has_exporter {
        return None;
    }

    let console = match var(CONSOLE_ENV).as_deref().map(str::trim) {
        None | Some("" | "capture") => OtelConsoleConfig::Capture,
        Some("replace") => OtelConsoleConfig::Replace,
        Some("ignore") => OtelConsoleConfig::Ignore,
        Some(other) => {
            tracing::warn!("Ignoring {CONSOLE_ENV}={other}: expected capture, replace, or ignore");
            OtelConsoleConfig::Capture
        }
    };

    Some(OtelConfig {
        tracing_enabled: true,
        metrics_enabled: true,
        console,
        deterministic_prefix: None,
        propagators: propagators(var("OTEL_PROPAGATORS").as_deref()),
    })
}

fn propagators(value: Option<&str>) -> HashSet<OtelPropagators> {
    let value = value.map(str::trim).filter(|value| !value.is_empty());
    value
        .unwrap_or("tracecontext,baggage")
        .split(',')
        .filter_map(|name| match name.trim() {
            "tracecontext" => Some(OtelPropagators::TraceContext),
            "baggage" => Some(OtelPropagators::Baggage),
            "none" => None,
            other => {
                tracing::warn!("Ignoring unsupported OpenTelemetry propagator {other:?}");
                None
            }
        })
        .collect()
}

#[cfg(test)]
#[expect(clippy::expect_used)]
mod tests {
    use rustc_hash::FxHashMap;

    use super::*;

    fn config(vars: &[(&str, &str)]) -> Option<OtelConfig> {
        let vars: FxHashMap<_, _> = vars.iter().copied().collect();
        config_from_env(|key| vars.get(key).map(ToString::to_string))
    }

    #[test]
    fn off_without_an_exporter() {
        assert!(config(&[]).is_none());
        assert!(config(&[("OTEL_EXPORTER_OTLP_ENDPOINT", " ")]).is_none());
        assert!(config(&[("OTEL_PROPAGATORS", "tracecontext")]).is_none());
    }

    #[test]
    fn on_with_an_endpoint_or_console_exporter() {
        let config =
            config(&[("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4318")]).expect("enabled");
        assert!(config.tracing_enabled && config.metrics_enabled);
        assert_eq!(config.console, OtelConsoleConfig::Capture);
        assert_eq!(
            config.propagators,
            HashSet::from([OtelPropagators::TraceContext, OtelPropagators::Baggage])
        );

        assert!(
            config_from_env(|key| {
                (key == "OTEL_EXPORTER_OTLP_PROTOCOL").then(|| "console".to_string())
            })
            .is_some()
        );
    }

    #[test]
    fn sdk_disabled_wins() {
        assert!(
            config(&[
                ("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4318"),
                ("OTEL_SDK_DISABLED", "TRUE"),
            ])
            .is_none()
        );
    }

    #[test]
    fn console_and_propagator_settings() {
        let config = config(&[
            ("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4318"),
            ("RARI_OTEL_CONSOLE", "ignore"),
            ("OTEL_PROPAGATORS", "none"),
        ])
        .expect("enabled");
        assert_eq!(config.console, OtelConsoleConfig::Ignore);
        assert!(config.propagators.is_empty());
    }
}
