# Changelog

What each release changed for you, newest first. Each line is a commit's summary, linked to its full description and diff. Releases before 3.0.3 are described by their release commits.

## 3.0.30 - 2026-11-11

### Fixes

- Require vpndetection 5.12.1: the free plan's daily trial, never a trial key ([`a9a5083`](https://github.com/vpndetection-io/sdk-rust-axum/commit/a9a5083b7d7d43d4aadefbd65c1aa0441039ab20))

## 3.0.29 - 2026-11-11

### Features

- Require vpndetection 5.12.0: trial_expired on a trial key's answers ([`e8d52ff`](https://github.com/vpndetection-io/sdk-rust-axum/commit/e8d52ff4bc56e7964239fbf8225ff5ac8b68f34f))

## 3.0.28 - 2026-11-11

### Fixes

- Require vpndetection 5.11.2: the Unlimited tier and its terms ([`c3687c7`](https://github.com/vpndetection-io/sdk-rust-axum/commit/c3687c77b3694195471fbf0d85f6534bc51ee15d))

## 3.0.27 - 2026-11-11

### Fixes

- Require vpndetection 5.11.1: plainer words on lookups without a key ([`f92c23e`](https://github.com/vpndetection-io/sdk-rust-axum/commit/f92c23e0708c901cb28e730656990cb87e0476db))

## 3.0.26 - 2026-11-11

### Features

- Require vpndetection 5.11.0: a lookup without a key answers more ([`9301ee2`](https://github.com/vpndetection-io/sdk-rust-axum/commit/9301ee260b353562bb527e71e0d33a83171088ff))

## 3.0.25 - 2026-11-07

### Features

- Require vpndetection 5.10.0: downloads take the build a webhook names ([`31ec68d`](https://github.com/vpndetection-io/sdk-rust-axum/commit/31ec68d745c51994dc86ddbd090f49623e36458f))

## 3.0.24 - 2026-11-04

### Fixes

- Require vpndetection 5.9.6, which fails only the address of a batch entry it cannot read ([`ba0d9f3`](https://github.com/vpndetection-io/sdk-rust-axum/commit/ba0d9f33735aec86ff49fac9edb69faeb8503767))

## 3.0.23 - 2026-11-02

### Fixes

- Require vpndetection 5.9.5: a metadata sample row that is no object is refused ([`1dc6999`](https://github.com/vpndetection-io/sdk-rust-axum/commit/1dc6999400a03c7316d12dbf6502599749110f73))

## 3.0.22 - 2026-10-30

### Fixes

- Require vpndetection 5.9.4: a lowercase t or z in a date-time reads ([`28d5b89`](https://github.com/vpndetection-io/sdk-rust-axum/commit/28d5b890589a0c04f298422119352d28354a845c))

## 3.0.21 - 2026-10-26

### Fixes

- Require vpndetection 5.9.3: re-pinned to the 2026.10.25 spec ([`2f561de`](https://github.com/vpndetection-io/sdk-rust-axum/commit/2f561de1ca2b2221e150d8ca3f8170f1d58759ec))

## 3.0.20 - 2026-10-24

### Fixes

- Require vpndetection 5.9.2, which requires tokio 1.53.3 ([`4f5103d`](https://github.com/vpndetection-io/sdk-rust-axum/commit/4f5103d3da40d989766577f4bbacbad36729f1d3))

## 3.0.19 - 2026-10-23

### Fixes

- Require vpndetection 5.9.1, which reads dates and Retry-After strictly ([`37e5c01`](https://github.com/vpndetection-io/sdk-rust-axum/commit/37e5c01bf1c90306920e7295fc85e79b2188585e))
- Test the layer built as the README builds it ([`c834ed0`](https://github.com/vpndetection-io/sdk-rust-axum/commit/c834ed01907dca3a26e4a9280e1b599498865fbf))

## 3.0.18 - 2026-10-21

### Fixes

- Require vpndetection 5.9.0, which adds oauth().register ([`7c7b468`](https://github.com/vpndetection-io/sdk-rust-axum/commit/7c7b468df8957a827053a0e3f50d7960691067db))

## 3.0.17 - 2026-10-20

### Features

- Require vpndetection 5.8.0: lookup results carry location and asn ([`2751c4e`](https://github.com/vpndetection-io/sdk-rust-axum/commit/2751c4e0a06e855f15795e71cb0069fae1843ff9))

## 3.0.16 - 2026-10-19

### Fixes

- Require vpndetection 5.7.0: num_ports and num_proxies in the pubproxy detail ([`6f64304`](https://github.com/vpndetection-io/sdk-rust-axum/commit/6f643047c647843e44fdef8ff4b3b819bcded530))

## 3.0.15 - 2026-10-18

### Fixes

- Require vpndetection 5.6.0: is_pubproxy and the pubproxy detail ([`d1e1ca0`](https://github.com/vpndetection-io/sdk-rust-axum/commit/d1e1ca0e0067324e8777d4d19a19733ac1da85dd))

## 3.0.14 - 2026-10-17

### Fixes

- Require vpndetection 5.5.0: builds beside reqwest 0.13.2 and newer ([`8e5f29a`](https://github.com/vpndetection-io/sdk-rust-axum/commit/8e5f29ab17f01075b9c4525c4ca83d5bfb590e5a))

## 3.0.13 - 2026-10-16

### Fixes

- Require vpndetection 5.4.3: the spec re-pinned to 2026.10.15 ([`b9fbcb8`](https://github.com/vpndetection-io/sdk-rust-axum/commit/b9fbcb8d0be996214db3ddca623350855a010776))

## 3.0.12 - 2026-10-10

### Fixes

- Require vpndetection 5.4.2: the spec re-pinned to 2026.10.09 ([`9715f65`](https://github.com/vpndetection-io/sdk-rust-axum/commit/9715f65a98d21615ab368361f633cdfed0fbd1d6))

## 3.0.11 - 2026-10-09

### Fixes

- Require vpndetection 5.4.1: an unreadable download link is retried ([`fe3bfb6`](https://github.com/vpndetection-io/sdk-rust-axum/commit/fe3bfb6f53faa7eb745b50d613211029e4fb9d4a))

## 3.0.10 - 2026-10-05

### Features

- Require vpndetection 5.4.0: the authorization code sign-in ([`361532e`](https://github.com/vpndetection-io/sdk-rust-axum/commit/361532e43ccd24fd17de9a369c937202f72751d3))

## 3.0.9 - 2026-10-04

### Fixes

- Require vpndetection 5.3.4, re-pinned to spec 2026.10.03 ([`70e29f1`](https://github.com/vpndetection-io/sdk-rust-axum/commit/70e29f1f59ae3556fc92e9d042e6afea3cf68c03))

## 3.0.8 - 2026-10-02

### Fixes

- Require vpndetection 5.3.3: tokio and chrono floors past their advisories ([`db21ce8`](https://github.com/vpndetection-io/sdk-rust-axum/commit/db21ce80ba3a2d136679996157cac873dcbcb89f))

## 3.0.7 - 2026-09-29

### Fixes

- Require vpndetection 5.3.2: 26 more reserved ranges are answered locally ([`2c8b655`](https://github.com/vpndetection-io/sdk-rust-axum/commit/2c8b655e3258061f1ed091aa40a577f245b85810))

## 3.0.6 - 2026-09-28

### Fixes

- Require vpndetection 5.3.1: IPv4-mapped visitors are looked up, not waved through ([`b968999`](https://github.com/vpndetection-io/sdk-rust-axum/commit/b96899925254e1639e92de2fecb52f17decd4562))

## 3.0.5 - 2026-09-27

### Features

- Require vpndetection 5.3.0: OauthMetadata carries client_id_metadata_document_supported ([`d2c76e0`](https://github.com/vpndetection-io/sdk-rust-axum/commit/d2c76e07037b7320f6f8da798a64b0ab970b13ca))

## 3.0.4 - 2026-09-26

### Fixes

- Require vpndetection 5.2.4: download_bytes no longer aborts the process ([`7fe52a8`](https://github.com/vpndetection-io/sdk-rust-axum/commit/7fe52a8acb043fe78251c62e06e458780da9decb))

## 3.0.3 - 2026-09-25

### Fixes

- Require vpndetection 5.2.3 ([`d7721fb`](https://github.com/vpndetection-io/sdk-rust-axum/commit/d7721fb0d7408bd8f23651af28fa329a8f7066df))
