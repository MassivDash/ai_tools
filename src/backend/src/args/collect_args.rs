/// Get the additional arguments from "cargo run"
///
/// List of arguments
/// Bind actix server to a host, used for development and production
/// --host=127.0.0.1
///
/// Bind actix server to a port, used for development and production
/// --port=8080
///
/// Set the environment
/// --env=prod / dev
///
/// Set the cors origin
/// --cors_url=astrox.spaceout.pl
///
/// Set the cookie domain
/// --cookie_domain=spaceout.pl
///
/// Set the ChromaDB address
/// --chroma_address=http://localhost:8000
pub struct Args {
    pub host: String,
    pub port: String,
    pub env: String,
    pub cors_url: String,
    pub chroma_address: Option<String>,
    pub llama_host: Option<String>,
    pub llama_port: Option<u16>,
}

pub fn collect_args(args: Vec<String>) -> Args {
    let mut env = "dev";
    let mut host = "127.0.0.1";
    let mut port = 8080;
    let mut cors_url = "astrox.spaceout.pl";
    let mut chroma_address: Option<String> = None;
    let mut llama_host: Option<String> = None;
    let mut llama_port: Option<u16> = None;

    for arg in &args {
        if arg.starts_with("--env=") {
            let split: Vec<&str> = arg.split('=').collect();
            if split.len() == 2 {
                env = split[1];
            }
        }

        if arg.starts_with("--host=") {
            let split: Vec<&str> = arg.split('=').collect();
            if split.len() == 2 {
                host = split[1];
            }
        }

        if arg.starts_with("--port=") {
            let split: Vec<&str> = arg.split('=').collect();
            if split.len() == 2 {
                port = split[1].parse::<u16>().unwrap();
            }
        }

        if arg.starts_with("--cors_url=") {
            let split: Vec<&str> = arg.split('=').collect();
            if split.len() == 2 {
                cors_url = split[1];
            }
        }

        if arg.starts_with("--chroma_address=") {
            let split: Vec<&str> = arg.split('=').collect();
            if split.len() == 2 && !split[1].is_empty() {
                chroma_address = Some(split[1].to_string());
            }
        }
        if arg.starts_with("--llama_host=") {
            let split: Vec<&str> = arg.split('=').collect();
            if split.len() == 2 && !split[1].is_empty() {
                llama_host = Some(split[1].to_string());
            }
        }

        if arg.starts_with("--llama_port=") {
            let split: Vec<&str> = arg.split('=').collect();
            if split.len() == 2 {
                if let Ok(p) = split[1].parse::<u16>() {
                    llama_port = Some(p);
                }
            }
        }
    }

    Args {
        host: host.to_string(),
        port: port.to_string(),
        env: env.to_string(),
        cors_url: cors_url.to_string(),
        chroma_address,
        llama_host,
        llama_port,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_collect_args_default() {
        let args = collect_args(env::args().collect());

        assert_eq!(args.host, "127.0.0.1");
        assert_eq!(args.port, "8080");
        assert_eq!(args.env, "dev");
    }

    #[test]
    fn test_collect_prod_arg() {
        let test_args = vec![
            "--env=prod".to_string(),
            "--port=4000".to_string(),
            "--host=0.0.0.0".to_string(),
            "--cors_url=spaceout.pl".to_string(),
        ];
        let args = collect_args(test_args);

        assert_eq!(args.host, "0.0.0.0");
        assert_eq!(args.port, "4000");
        assert_eq!(args.env, "prod");
        assert_eq!(args.cors_url, "spaceout.pl");
    }

    #[test]
    fn test_collect_optional_args_are_none_by_default() {
        let args = collect_args(vec!["--env=dev".to_string()]);

        assert!(args.chroma_address.is_none());
        assert!(args.llama_host.is_none());
        assert!(args.llama_port.is_none());
    }

    #[test]
    fn test_collect_chroma_and_llama_args() {
        let args = collect_args(vec![
            "--chroma_address=http://localhost:8000".to_string(),
            "--llama_host=0.0.0.0".to_string(),
            "--llama_port=8090".to_string(),
        ]);

        assert_eq!(
            args.chroma_address,
            Some("http://localhost:8000".to_string())
        );
        assert_eq!(args.llama_host, Some("0.0.0.0".to_string()));
        assert_eq!(args.llama_port, Some(8090));
    }

    #[test]
    fn test_empty_optional_values_are_ignored() {
        let args = collect_args(vec![
            "--chroma_address=".to_string(),
            "--llama_host=".to_string(),
        ]);

        assert!(args.chroma_address.is_none());
        assert!(args.llama_host.is_none());
    }

    #[test]
    fn test_unparseable_llama_port_is_ignored() {
        let args = collect_args(vec!["--llama_port=not-a-number".to_string()]);

        assert!(args.llama_port.is_none());
    }

    #[test]
    fn test_llama_port_out_of_u16_range_is_ignored() {
        let args = collect_args(vec!["--llama_port=99999".to_string()]);

        assert!(args.llama_port.is_none());
    }

    #[test]
    fn test_values_containing_extra_equals_are_ignored() {
        // The parser splits on every '=' and only accepts exactly two parts,
        // so a value containing '=' is dropped and the default is kept.
        let args = collect_args(vec![
            "--host=a=b".to_string(),
            "--chroma_address=http://x/?a=b".to_string(),
        ]);

        assert_eq!(args.host, "127.0.0.1");
        assert!(args.chroma_address.is_none());
    }

    #[test]
    fn test_last_occurrence_of_a_flag_wins() {
        let args = collect_args(vec![
            "--port=4000".to_string(),
            "--port=5000".to_string(),
            "--llama_port=1".to_string(),
            "--llama_port=2".to_string(),
        ]);

        assert_eq!(args.port, "5000");
        assert_eq!(args.llama_port, Some(2));
    }

    #[test]
    fn test_unrelated_args_do_not_change_defaults() {
        let args = collect_args(vec![
            "target/debug/backend".to_string(),
            "--nonsense".to_string(),
            "host=1.2.3.4".to_string(),
        ]);

        assert_eq!(args.host, "127.0.0.1");
        assert_eq!(args.port, "8080");
        assert_eq!(args.env, "dev");
        assert_eq!(args.cors_url, "astrox.spaceout.pl");
    }
}
