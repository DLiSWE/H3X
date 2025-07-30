// This file generates a self-signed TLS certificate for use in testing or local development.
// It uses the `rcgen` crate to create the certificate and `quinn` for the TLS types.
// The generated certificate is suitable for use with the `quinn` library.
// Returns a tuple containing the `Certificate` and `PrivateKey`.
// This code is intended for use in a Rust project that requires TLS functionality.
// The `generate_cert` function can be called to obtain a self-signed certificate and private key.
// Usage:
// let (cert, key) = generate_cert();   
// This is used in the context of a QUIC server.

use rcgen::{generate_simple_self_signed};
use rustls::{Certificate, PrivateKey};
use std::fs;
use std::path::Path;

const CERT_PATH: &str = "cert.der";
const KEY_PATH: &str = "key.der";

pub fn generate_or_load_cert() -> (Vec<Certificate>, PrivateKey) {
    if Path::new(CERT_PATH).exists() && Path::new(KEY_PATH).exists() {
        let cert = fs::read(CERT_PATH).unwrap();
        let key = fs::read(KEY_PATH).unwrap();
        return (vec![Certificate(cert)], PrivateKey(key));
    }

    let rcgen_cert = generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let cert_der = rcgen_cert.serialize_der().unwrap();
    let key_der = rcgen_cert.serialize_private_key_der();

    fs::write(CERT_PATH, &cert_der).unwrap();
    fs::write(KEY_PATH, &key_der).unwrap();

    (vec![Certificate(cert_der)], PrivateKey(key_der))
}
