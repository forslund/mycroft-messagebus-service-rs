# Rust implementation of the Mycroft Messagebus Service

A very simple implementation of the websocket service used by [Mycroft-Core](https://github.com/MycroftAI/mycroft-core) in Rust.

This version can be used as a drop-in replacement for the service included in Mycroft-core. Currently it doesn't handle the ws over TLS but the default messagebus config doesn't use this feature.

## TODO

- TLS-support
- Handle a complete mycroft config "stack"
