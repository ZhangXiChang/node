use std::sync::{Arc, Mutex, MutexGuard};

use eyre::{eyre, Result};
use x509_parser::{
    certificate::X509Certificate,
    der_parser::asn1_rs::FromDer,
    extensions::{GeneralName, ParsedExtension},
};

pub struct ArcMutex<T> {
    arc_t: Arc<Mutex<T>>,
}
impl<T> ArcMutex<T> {
    pub fn new(t: T) -> Self {
        Self {
            arc_t: Arc::new(Mutex::new(t)),
        }
    }
    pub fn lock(&self) -> MutexGuard<T> {
        self.arc_t.lock().unwrap()
    }
}
impl<T> Clone for ArcMutex<T> {
    fn clone(&self) -> Self {
        Self {
            arc_t: self.arc_t.clone(),
        }
    }
}

pub fn x509_dns_name_from_der(cert_bytes: &[u8]) -> Result<String> {
    for x509extension in X509Certificate::from_der(cert_bytes)?
        .1
        .tbs_certificate
        .extensions()
    {
        if let ParsedExtension::SubjectAlternativeName(names) = x509extension.parsed_extension() {
            for name in names.general_names.iter() {
                if let GeneralName::DNSName(dns_name) = name {
                    return Ok(dns_name.to_string());
                }
            }
        }
    }
    return Err(eyre!("读取DnsName字段失败"));
}
