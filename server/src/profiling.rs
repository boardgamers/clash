#[cfg(not(feature = "profiling"))]
pub fn start_profiling() {
    // do nothing
}

#[cfg(feature = "profiling")]
pub fn start_profiling() {
    println!("start profiling");

    use pyroscope::PyroscopeAgent;
    use pyroscope_pprofrs::{PprofConfig, pprof_backend};

    let pprof_config = PprofConfig::new().sample_rate(100);
    let backend_impl = pprof_backend(pprof_config);

    let agent = PyroscopeAgent::builder("http://localhost:4040", "clash")
        .backend(backend_impl)
        .build()
        .expect("Failed to initialize pyroscope");
    let _ = agent.start().unwrap();
}

