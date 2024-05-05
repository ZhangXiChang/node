use eyre::{eyre, Result};
use x509_parser::{
    certificate::X509Certificate,
    der_parser::asn1_rs::FromDer,
    extensions::{GeneralName, ParsedExtension},
};

pub fn x509_dns_name_from_cert_der(cert_der: Vec<u8>) -> Result<String> {
    for x509extension in X509Certificate::from_der(&cert_der)?
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
