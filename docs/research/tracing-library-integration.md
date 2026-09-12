# Tracing library integration

The viewer library depends on `tracing` and emits events through its macros. Subscriber setup belongs to the executable. The tracing documentation explicitly separates library instrumentation from executable collection and advises libraries against setting a global subscriber because it can conflict with application setup. [Official tracing documentation](https://docs.rs/tracing/latest/tracing/#in-libraries)

The executable chooses filtering and output. Subscribers decide which events to collect; events outside a subscriber's context are not collected. [Subscribers](https://docs.rs/tracing/latest/tracing/#subscribers), [executable integration](https://docs.rs/tracing/latest/tracing/#in-executables)

For this viewer, the agreed application policy is stderr logging under `--debug`, with configuration and loading errors reported independently of logging. The library receives no logger argument and stores no logger handle. Its returned descriptions carry structured problems with MIB data, independently of event collection.
