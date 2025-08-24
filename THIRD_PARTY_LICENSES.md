# Third Party Licenses

This extension bundles and redistributes the following third-party software:

## PHP_CodeSniffer

**Source:** https://github.com/PHPCSStandards/PHP_CodeSniffer  
**License:** BSD-3-Clause  
**Usage:** Bundled as `phpcs.phar` and `phpcbf.phar` binaries

```
Copyright (c) 2012, PHP_CodeSniffer (Squizlabs)
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

    * Redistributions of source code must retain the above copyright
      notice, this list of conditions and the following disclaimer.
    * Redistributions in binary form must reproduce the above copyright
      notice, this list of conditions and the following disclaimer in the
      documentation and/or other materials provided with the distribution.
    * Neither the name of Squizlabs nor the names of its contributors may be
      used to endorse or promote products derived from this software without
      specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND
ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR
ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
(INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON
ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```

## Rust Dependencies

This extension is built using Rust and includes the following dependencies, all with permissive licenses compatible with this project's MIT license:

### (Apache-2.0 OR MIT) AND Unicode-3.0

- **unicode-ident** v1.0.18 - https://github.com/dtolnay/unicode-ident

### 0BSD OR Apache-2.0 OR MIT

- **adler2** v2.0.1 - https://github.com/oyvindln/adler2

### Apache-2.0

- **zed_extension_api** v0.6.0 - https://github.com/zed-industries/zed

### Apache-2.0 OR Apache-2.0 WITH LLVM-exception OR MIT

- **linux-raw-sys** v0.9.4 - https://github.com/sunfishcode/linux-raw-sys
- **rustix** v1.0.8 - https://github.com/bytecodealliance/rustix
- **wasi** v0.14.2+wasi-0.2.4 - https://github.com/bytecodealliance/wasi-rs
- **wasm-encoder** v0.227.1 - https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wasm-encoder
- **wasm-metadata** v0.227.1 - https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wasm-metadata
- **wasmparser** v0.227.1 - https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wasmparser
- **wit-bindgen** v0.41.0 - https://github.com/bytecodealliance/wit-bindgen
- **wit-bindgen-core** v0.41.0 - https://github.com/bytecodealliance/wit-bindgen
- **wit-bindgen-rt** v0.39.0 - https://github.com/bytecodealliance/wit-bindgen
- **wit-bindgen-rt** v0.41.0 - https://github.com/bytecodealliance/wit-bindgen
- **wit-bindgen-rust** v0.41.0 - https://github.com/bytecodealliance/wit-bindgen
- **wit-bindgen-rust-macro** v0.41.0 - https://github.com/bytecodealliance/wit-bindgen
- **wit-component** v0.227.1 - https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wit-component
- **wit-parser** v0.227.1 - https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wit-parser

### Apache-2.0 OR BSL-1.0

- **ryu** v1.0.20 - https://github.com/dtolnay/ryu

### Apache-2.0 OR LGPL-2.1-or-later OR MIT

- **r-efi** v5.3.0 - https://github.com/r-efi/r-efi

### Apache-2.0 OR MIT

- **anyhow** v1.0.99 - https://github.com/dtolnay/anyhow
- **auditable-serde** v0.8.0 - https://github.com/rust-secure-code/cargo-auditable
- **bitflags** v2.9.3 - https://github.com/bitflags/bitflags
- **cfg-if** v1.0.3 - https://github.com/rust-lang/cfg-if
- **crc32fast** v1.5.0 - https://github.com/srijs/rust-crc32fast
- **displaydoc** v0.2.5 - https://github.com/yaahc/displaydoc
- **equivalent** v1.0.2 - https://github.com/indexmap-rs/equivalent
- **errno** v0.3.13 - https://github.com/lambda-fairy/rust-errno
- **fastrand** v2.3.0 - https://github.com/smol-rs/fastrand
- **flate2** v1.1.2 - https://github.com/rust-lang/flate2-rs
- **form_urlencoded** v1.2.2 - https://github.com/servo/rust-url
- **futures** v0.3.31 - https://github.com/rust-lang/futures-rs
- **futures-channel** v0.3.31 - https://github.com/rust-lang/futures-rs
- **futures-core** v0.3.31 - https://github.com/rust-lang/futures-rs
- **futures-executor** v0.3.31 - https://github.com/rust-lang/futures-rs
- **futures-io** v0.3.31 - https://github.com/rust-lang/futures-rs
- **futures-macro** v0.3.31 - https://github.com/rust-lang/futures-rs
- **futures-sink** v0.3.31 - https://github.com/rust-lang/futures-rs
- **futures-task** v0.3.31 - https://github.com/rust-lang/futures-rs
- **futures-util** v0.3.31 - https://github.com/rust-lang/futures-rs
- **getrandom** v0.3.3 - https://github.com/rust-random/getrandom
- **hashbrown** v0.15.5 - https://github.com/rust-lang/hashbrown
- **heck** v0.5.0 - https://github.com/withoutboats/heck
- **id-arena** v2.2.1 - https://github.com/fitzgen/id-arena
- **idna** v1.1.0 - https://github.com/servo/rust-url/
- **idna_adapter** v1.2.1 - https://github.com/hsivonen/idna_adapter
- **indexmap** v2.11.0 - https://github.com/indexmap-rs/indexmap
- **itoa** v1.0.15 - https://github.com/dtolnay/itoa
- **leb128fmt** v0.1.0 - https://github.com/bluk/leb128fmt
- **libc** v0.2.175 - https://github.com/rust-lang/libc
- **log** v0.4.27 - https://github.com/rust-lang/log
- **once_cell** v1.21.3 - https://github.com/matklad/once_cell
- **percent-encoding** v2.3.2 - https://github.com/servo/rust-url/
- **pin-project-lite** v0.2.16 - https://github.com/taiki-e/pin-project-lite
- **pin-utils** v0.1.0 - https://github.com/rust-lang-nursery/pin-utils
- **prettyplease** v0.2.37 - https://github.com/dtolnay/prettyplease
- **proc-macro2** v1.0.101 - https://github.com/dtolnay/proc-macro2
- **quote** v1.0.40 - https://github.com/dtolnay/quote
- **semver** v1.0.26 - https://github.com/dtolnay/semver
- **serde** v1.0.219 - https://github.com/serde-rs/serde
- **serde_derive** v1.0.219 - https://github.com/serde-rs/serde
- **serde_json** v1.0.143 - https://github.com/serde-rs/json
- **smallvec** v1.15.1 - https://github.com/servo/rust-smallvec
- **spdx** v0.10.9 - https://github.com/EmbarkStudios/spdx
- **stable_deref_trait** v1.2.0 - https://github.com/storyyeller/stable_deref_trait
- **syn** v2.0.106 - https://github.com/dtolnay/syn
- **tempfile** v3.21.0 - https://github.com/Stebalien/tempfile
- **topological-sort** v0.2.2 - https://github.com/gifnksm/topological-sort-rs
- **unicode-xid** v0.2.6 - https://github.com/unicode-rs/unicode-xid
- **url** v2.5.7 - https://github.com/servo/rust-url
- **utf8_iter** v1.0.4 - https://github.com/hsivonen/utf8_iter
- **windows-link** v0.1.3 - https://github.com/microsoft/windows-rs
- **windows-sys** v0.60.2 - https://github.com/microsoft/windows-rs
- **windows-targets** v0.53.3 - https://github.com/microsoft/windows-rs
- **windows_aarch64_gnullvm** v0.53.0 - https://github.com/microsoft/windows-rs
- **windows_aarch64_msvc** v0.53.0 - https://github.com/microsoft/windows-rs
- **windows_i686_gnu** v0.53.0 - https://github.com/microsoft/windows-rs
- **windows_i686_gnullvm** v0.53.0 - https://github.com/microsoft/windows-rs
- **windows_i686_msvc** v0.53.0 - https://github.com/microsoft/windows-rs
- **windows_x86_64_gnu** v0.53.0 - https://github.com/microsoft/windows-rs
- **windows_x86_64_gnullvm** v0.53.0 - https://github.com/microsoft/windows-rs
- **windows_x86_64_msvc** v0.53.0 - https://github.com/microsoft/windows-rs

### Apache-2.0 OR MIT OR Zlib

- **miniz_oxide** v0.8.9 - https://github.com/Frommi/miniz_oxide/tree/master/miniz_oxide

### MIT

- **slab** v0.4.11 - https://github.com/tokio-rs/slab
- **synstructure** v0.13.2 - https://github.com/mystor/synstructure

### MIT OR Unlicense

- **memchr** v2.7.5 - https://github.com/BurntSushi/memchr

### Unicode-3.0

- **icu_collections** v2.0.0 - https://github.com/unicode-org/icu4x
- **icu_locale_core** v2.0.0 - https://github.com/unicode-org/icu4x
- **icu_normalizer** v2.0.0 - https://github.com/unicode-org/icu4x
- **icu_normalizer_data** v2.0.0 - https://github.com/unicode-org/icu4x
- **icu_properties** v2.0.1 - https://github.com/unicode-org/icu4x
- **icu_properties_data** v2.0.1 - https://github.com/unicode-org/icu4x
- **icu_provider** v2.0.0 - https://github.com/unicode-org/icu4x
- **litemap** v0.8.0 - https://github.com/unicode-org/icu4x
- **potential_utf** v0.1.2 - https://github.com/unicode-org/icu4x
- **tinystr** v0.8.1 - https://github.com/unicode-org/icu4x
- **writeable** v0.6.1 - https://github.com/unicode-org/icu4x
- **yoke** v0.8.0 - https://github.com/unicode-org/icu4x
- **yoke-derive** v0.8.0 - https://github.com/unicode-org/icu4x
- **zerofrom** v0.1.6 - https://github.com/unicode-org/icu4x
- **zerofrom-derive** v0.1.6 - https://github.com/unicode-org/icu4x
- **zerotrie** v0.2.2 - https://github.com/unicode-org/icu4x
- **zerovec** v0.11.4 - https://github.com/unicode-org/icu4x
- **zerovec-derive** v0.11.1 - https://github.com/unicode-org/icu4x

### Zlib

- **foldhash** v0.1.5 - https://github.com/orlp/foldhash