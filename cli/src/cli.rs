use clap::Parser;
use std::path::Path;

#[derive(Parser)]
#[command(name = "mapodus", about = "Convert Google Maps saved places to uMap")]
pub struct CliArgs {
    #[arg(short = 't', long, help = "Path to Google Takeout CSV or JSON file")]
    pub takeout: Option<String>,

    #[arg(
        short = 'g',
        long,
        help = "Path to existing GeoJSON file (alternative to --takeout)"
    )]
    pub geojson: Option<String>,

    #[arg(
        short = 'o',
        long,
        help = "Output GeoJSON file path (skip uMap upload)"
    )]
    pub output: Option<String>,

    #[arg(
        long,
        default_value = "https://umap.openstreetmap.fr/en/",
        help = "uMap instance URL"
    )]
    pub umap_url: String,

    #[arg(long, help = "uMap map ID to upload to")]
    pub umap_map_id: Option<String>,

    #[arg(long, help = "Create a new map with this name before uploading")]
    pub create_map: Option<String>,

    #[arg(
        long,
        help = "uMap session cookie (format: sessionid=xxx; csrftoken=xxx)"
    )]
    pub umap_cookie: Option<String>,

    #[arg(long, default_value = "Google Maps Saved", help = "Target layer name")]
    pub layer_name: String,
}

impl CliArgs {
    pub fn validate(&self) -> Result<(), anyhow::Error> {
        let has_takeout = self.takeout.is_some();
        let has_geojson = self.geojson.is_some();

        if !has_takeout && !has_geojson {
            return Err(anyhow::anyhow!(
                "Either --takeout or --geojson must be provided"
            ));
        }

        if has_takeout && has_geojson {
            return Err(anyhow::anyhow!(
                "Only one of --takeout or --geojson can be provided"
            ));
        }

        if self.create_map.is_some() && self.umap_cookie.is_none() {
            return Err(anyhow::anyhow!(
                "--umap-cookie must be provided when --create-map is specified"
            ));
        }

        if self.create_map.is_some() && self.umap_map_id.is_some() {
            return Err(anyhow::anyhow!(
                "--create-map and --umap-map-id are mutually exclusive"
            ));
        }

        if self.umap_map_id.is_some() && self.umap_cookie.is_none() {
            return Err(anyhow::anyhow!(
                "--umap-cookie must be provided when --umap-map-id is specified"
            ));
        }

        if let Some(ref path) = self.takeout
            && !Path::new(path).exists()
        {
            return Err(anyhow::anyhow!("Takeout file does not exist: {}", path));
        }

        if let Some(ref path) = self.geojson
            && !Path::new(path).exists()
        {
            return Err(anyhow::anyhow!("GeoJSON file does not exist: {}", path));
        }

        if let Some(ref path) = self.output {
            let parent = Path::new(path).parent().unwrap_or(Path::new("."));
            if !parent.exists() {
                return Err(anyhow::anyhow!(
                    "Output directory does not exist: {:?}",
                    parent
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn examples_dir() -> String {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("examples")
            .to_string_lossy()
            .to_string()
    }

    fn make_args(
        takeout: Option<&str>,
        geojson: Option<&str>,
        create_map: Option<&str>,
        umap_map_id: Option<&str>,
        umap_cookie: Option<&str>,
        output: Option<&str>,
    ) -> CliArgs {
        CliArgs {
            takeout: takeout.map(|s| s.to_string()),
            geojson: geojson.map(|s| s.to_string()),
            output: output.map(|s| s.to_string()),
            umap_url: "http://localhost:8000/".to_string(),
            umap_map_id: umap_map_id.map(|s| s.to_string()),
            create_map: create_map.map(|s| s.to_string()),
            umap_cookie: umap_cookie.map(|s| s.to_string()),
            layer_name: "Test Layer".to_string(),
        }
    }

    #[test]
    fn validate_fails_with_neither_takeout_nor_geojson() {
        let args = make_args(None, None, None, None, None, None);
        assert!(args.validate().is_err());
        assert!(
            args.validate()
                .unwrap_err()
                .to_string()
                .contains("--takeout or --geojson")
        );
    }

    #[test]
    fn validate_fails_with_both_takeout_and_geojson() {
        let csv = format!("{}/test.csv", examples_dir());
        let args = make_args(Some(&csv), Some(&csv), None, None, None, None);
        assert!(args.validate().is_err());
        assert!(
            args.validate()
                .unwrap_err()
                .to_string()
                .contains("Only one of")
        );
    }

    #[test]
    fn validate_fails_when_create_map_without_cookie() {
        let csv = format!("{}/test.csv", examples_dir());
        let args = make_args(Some(&csv), None, Some("My Map"), None, None, None);
        assert!(args.validate().is_err());
        assert!(
            args.validate()
                .unwrap_err()
                .to_string()
                .contains("--umap-cookie must be provided")
        );
    }

    #[test]
    fn validate_fails_when_create_map_with_umap_map_id() {
        let csv = format!("{}/test.csv", examples_dir());
        let cookie = "sessionid=a; csrftoken=b";
        let args = make_args(
            Some(&csv),
            None,
            Some("My Map"),
            Some("map-123"),
            Some(cookie),
            None,
        );
        assert!(args.validate().is_err());
        assert!(
            args.validate()
                .unwrap_err()
                .to_string()
                .contains("mutually exclusive")
        );
    }

    #[test]
    fn validate_fails_when_umap_map_id_without_cookie() {
        let csv = format!("{}/test.csv", examples_dir());
        let args = make_args(Some(&csv), None, None, Some("map-123"), None, None);
        assert!(args.validate().is_err());
        assert!(
            args.validate()
                .unwrap_err()
                .to_string()
                .contains("--umap-cookie must be provided")
        );
    }

    #[test]
    fn validate_fails_when_takeout_does_not_exist() {
        let args = make_args(
            Some("/tmp/nonexistent-file.csv"),
            None,
            None,
            None,
            None,
            None,
        );
        assert!(args.validate().is_err());
        assert!(
            args.validate()
                .unwrap_err()
                .to_string()
                .contains("does not exist")
        );
    }

    #[test]
    fn validate_fails_when_geojson_does_not_exist() {
        let args = make_args(
            None,
            Some("/tmp/nonexistent.geojson"),
            None,
            None,
            None,
            None,
        );
        assert!(args.validate().is_err());
        assert!(
            args.validate()
                .unwrap_err()
                .to_string()
                .contains("does not exist")
        );
    }

    #[test]
    fn validate_fails_when_output_parent_does_not_exist() {
        let csv = format!("{}/test.csv", examples_dir());
        let args = make_args(
            Some(&csv),
            None,
            None,
            None,
            None,
            Some("/nonexistent-dir/output.geojson"),
        );
        assert!(args.validate().is_err());
        assert!(
            args.validate()
                .unwrap_err()
                .to_string()
                .contains("Output directory does not exist")
        );
    }

    #[test]
    fn validate_succeeds_with_valid_takeout() {
        let csv = format!("{}/test.csv", examples_dir());
        let args = make_args(Some(&csv), None, None, None, None, None);
        assert!(args.validate().is_ok());
    }

    #[test]
    fn validate_succeeds_with_takeout_and_output() {
        let csv = format!("{}/test.csv", examples_dir());
        let args = make_args(
            Some(&csv),
            None,
            None,
            None,
            None,
            Some("/tmp/output.geojson"),
        );
        assert!(args.validate().is_ok());
    }
}
