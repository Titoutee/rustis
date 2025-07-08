# Rustis
Rustis is a compact, thread-safe, babytoy Redis server implementing  part of *standard RESP*, written in pure Rust.

## Motivation
This project was firstly motivated by the **Codecrafters** related project, and was further developed away from CC's framework, and thus became a personal initiative.

[For documentation on RESP](https://redis.io/docs/latest/develop/reference/protocol-spec/)

[For information on Redis](https://en.wikipedia.org/wiki/Redis)

## Appendix

### How do I test it out?

Basically, take or craft any python client or *telnet* script to communicate with the given Redis server. As an example, `client.py` performs some payload tests to check for server responses. 

The default used addr/port config is **`localhost:6378`** but feel free to pick anyone you wish.