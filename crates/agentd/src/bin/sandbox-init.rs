//! `sandbox-init` — the container entrypoint that prepares the egress proxy
//! (CA trust + iptables NAT rules) and then exec()s into the actual workload.
//!
//! Usage (from a runtime-image Dockerfile):
//!
//! ```dockerfile
//! ENTRYPOINT ["tini", "--", "sandbox-init"]
//! CMD ["agentd"]
//! ```
//!
//! Reads `SANDBOX_EGRESS_PROXY`, `SANDBOX_EGRESS_PROXY_IP`, and
//! `SANDBOX_EGRESS_CA_PEM` from the environment. If none are set, exec()s
//! straight through (egress is opt-in). If `SANDBOX_EGRESS_PROXY` is set but
//! the others are missing, exits non-zero so a misconfigured drone is loud.

// stderr is the right channel here: tracing isn't initialised at bootstrap and
// these messages must be visible to `docker logs` before exec()ing the workload.
#![allow(clippy::print_stderr)]

use std::ffi::CString;
use std::os::unix::process::CommandExt;
use std::process::Command;

use agentd::egress;

fn main() -> ! {
    let mut args = std::env::args_os();
    let _self = args.next();
    let argv: Vec<_> = args.collect();
    if argv.is_empty() {
        eprintln!("sandbox-init: no command to exec");
        std::process::exit(64);
    }

    match egress::Config::from_env() {
        Ok(Some(cfg)) => {
            if let Err(e) = cfg.install() {
                eprintln!("sandbox-init: egress setup failed: {e}");
                std::process::exit(70);
            }
        }
        Ok(None) => {
            // Egress not configured — pass straight through.
        }
        Err(e) => {
            eprintln!("sandbox-init: bad egress env: {e}");
            std::process::exit(78);
        }
    }

    // Replace ourselves with the workload so PID-1 semantics, signal forwarding,
    // and zombie reaping (already provided by tini one layer up) work correctly.
    let program = argv[0].clone();
    let err = Command::new(&program).args(&argv[1..]).exec();
    // exec() only returns on failure.
    eprintln!(
        "sandbox-init: exec {:?} failed: {err}",
        CString::new(program.to_string_lossy().into_owned()).unwrap_or_default()
    );
    std::process::exit(126);
}
