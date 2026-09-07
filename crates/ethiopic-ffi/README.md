# `libethiopic`

C ABI over [`ethiopic-calendar`](../ethiopic-calendar). Produces
`libethiopic.so`, `libethiopic.a` and a cbindgen-generated
[`ethiopic.h`](include/ethiopic.h).

```c
EthDate d;
eth_from_gregorian(2024, 9, 11, &d);   /* -> 2017-01-01 E.C. */

char buf[128];
eth_format(&d, "%A %-d %B %Y %N", ETH_LOCALE_AMHARIC, buf, sizeof buf);
```

Contract: every fallible call returns an `EthStatus`; nothing unwinds; nothing
allocates on the caller's behalf (`eth_format` follows the `snprintf`
convention); all pointers may be `NULL`; no global mutable state.

See [`examples/demo.c`](examples/demo.c).
