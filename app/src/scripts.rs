use prometheus_client::{
    encoding::{EncodeMetric, MetricEncoder},
    metrics::MetricType,
    registry::Registry,
};

use std::{env, fs, path::PathBuf, process::Command};

pub fn get_registry() -> Registry {
    let mut registry = Registry::with_prefix(env::var("PL_NAME").unwrap());

    let folder =
        env::var("PL_SCRIPT_FOLDER").expect("PL_SCRIPT_FOLDER environment variable doesn't exist");

    log::debug!("Opening folder {}", folder);

    let files = fs::read_dir(folder).expect("PL_SCRIPT_FOLDER isn't reachable.");
    let mut counter: u64 = 0;

    for fe in files {
        let file = fe.unwrap();
        if !file.path().to_str().unwrap().ends_with(".prom") {
            continue;
        }

        log::debug!("Loading file {}", file.path().to_str().unwrap());

        let (label, metric_type, description, failure_value) = read_infos(file.path());

        registry.register(
            label,
            description,
            ScraperScript {
                script: file.path().to_str().unwrap().to_owned(),
                metric_type,
                failure_value,
            },
        );

        counter += 1;
    }
    log::info!("Loaded {} scripts", counter);

    registry
}

fn read_infos(path: PathBuf) -> (String, MetricType, String, f64) {
    let mut label: String = "".to_string();
    let mut metric_type: MetricType = MetricType::Unknown;
    let mut default_value: f64 = f64::NAN;
    let mut description: String = "".to_string();
    let mut type_text: String = "".to_string();
    let mut null_text: String = "".to_string();

    let contents = fs::read_to_string(path).unwrap();

    for line in contents.lines() {
        if !line.is_empty()
            && !description.is_empty()
            && !type_text.is_empty()
            && !null_text.is_empty()
        {
            break;
        }

        if line.starts_with("#label=") {
            label = line.split("=").last().unwrap().trim().to_owned();
        }
        if line.starts_with("#description=") {
            description = line.split("=").last().unwrap().trim().to_owned();
        }
        if line.starts_with("#type=") {
            type_text = line.split("=").last().unwrap().trim().to_lowercase();
            match type_text.as_str() {
                "counter" => metric_type = MetricType::Counter,
                "gauge" => metric_type = MetricType::Gauge,
                "histogram" => metric_type = MetricType::Histogram,
                "info" => metric_type = MetricType::Info,
                _ => metric_type = MetricType::Unknown,
            }
        }
        if line.starts_with("#null=") {
            null_text = line.split("=").last().unwrap().trim().to_owned();
            default_value = null_text.parse::<f64>().unwrap();
        }
    }

    (label, metric_type, description, default_value)
}

#[derive(Debug)]
struct ScraperScript {
    script: String,
    metric_type: MetricType,
    failure_value: f64,
}

impl EncodeMetric for ScraperScript {
    fn encode(&self, mut encoder: MetricEncoder) -> Result<(), std::fmt::Error> {
        // This method is called on each Prometheus server scrape. Allowing you
        // to execute whatever logic is needed to generate and encode your
        // custom metric.
        //
        // Do keep in mind that "with great power comes great responsibility".
        // E.g. every CPU cycle spend in this method delays the response send to
        // the Prometheus server.
        let command_result = Command::new(&self.script).output();

        if command_result.is_err() {
            log::warn!("Failed to run script {}", &self.script);

            return encoder.encode_counter::<(), _, f64>(&self.failure_value, None);
        }

        let command_output = command_result.unwrap();

        if !command_output.status.success() {
            log::error!(
                "{} has failed with return value {}",
                &self.script,
                command_output.status
            );
            return encoder.encode_counter::<(), _, f64>(&self.failure_value, None);
        }

        let stderr = String::from_utf8(command_output.stderr)
            .unwrap()
            .trim()
            .to_owned();
        if stderr != "" {
            log::warn!("{} has returned a non-empty stderr.", &self.script);
        }

        let binding = String::from_utf8(command_output.stdout)
            .unwrap();
        let output = binding.trim()
            .split('\n')
            .filter(|x| !x.starts_with('#'))
            .next()
            .to_owned();

        let measured_int = output.unwrap().parse::<u64>();
        let measured_float = output.unwrap().parse::<f64>();

        if measured_int.is_err() && measured_float.is_err() {
            log::error!(
                "{} stdout was not a valid unsigned integer or a valid float",
                &self.script
            );
            return encoder.encode_counter::<(), _, f64>(&self.failure_value, None);
        }

        if measured_int.is_ok() {
            return encoder.encode_counter::<(), _, u64>(&measured_int.unwrap(), None);
        } else {
            return encoder.encode_counter::<(), _, f64>(&measured_float.unwrap(), None);
        }
    }

    fn metric_type(&self) -> MetricType {
        self.metric_type
    }
}
