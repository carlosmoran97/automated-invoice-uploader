use serde_json::Value;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct InvoiceSummary {
    pub source_filename: String,
    pub document_type_code: String,
    pub document_type_name: String,
    pub is_ccf: bool,
    pub control_number: String,
    pub generation_code: String,
    pub issue_date: String,
    pub issue_time: String,
    pub currency: String,
    pub issuer_name: String,
    pub issuer_nit: String,
    pub issuer_nrc: String,
    pub receiver_name: String,
    pub receiver_nit: String,
    pub receiver_nrc: String,
    pub taxed_sales: String,
    pub exempt_sales: String,
    pub non_subject_sales: String,
    pub subtotal_sales: String,
    pub discount_total: String,
    pub subtotal: String,
    pub taxes: Vec<TaxSummary>,
    pub income_tax_retention: String,
    pub vat_retention: String,
    pub vat_perception: String,
    pub operation_total: String,
    pub total_to_pay: String,
    pub total_in_words: String,
    pub payment_condition: String,
    pub line_items: Vec<InvoiceLineItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TaxSummary {
    pub code: String,
    pub description: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InvoiceLineItem {
    pub description: String,
    pub quantity: String,
    pub unit_price: String,
    pub taxed_sale: String,
    pub exempt_sale: String,
    pub non_subject_sale: String,
}

#[derive(Debug)]
pub enum InvoiceParseError {
    InvalidJson(serde_json::Error),
    MissingObject,
}

impl InvoiceSummary {
    pub fn from_json_bytes(filename: &str, bytes: &[u8]) -> Result<Self, InvoiceParseError> {
        let value: Value = serde_json::from_slice(bytes).map_err(InvoiceParseError::InvalidJson)?;
        let object = value.as_object().ok_or(InvoiceParseError::MissingObject)?;
        let root = Value::Object(object.clone());

        let document_type_code = string_at(&root, &["identificacion", "tipoDte"]);
        let document_type_name = document_type_name(&document_type_code).to_string();

        Ok(Self {
            source_filename: filename.to_string(),
            is_ccf: document_type_code == "03",
            document_type_code,
            document_type_name,
            control_number: string_at(&root, &["identificacion", "numeroControl"]),
            generation_code: string_at(&root, &["identificacion", "codigoGeneracion"]),
            issue_date: string_at(&root, &["identificacion", "fecEmi"]),
            issue_time: string_at(&root, &["identificacion", "horEmi"]),
            currency: string_at(&root, &["identificacion", "tipoMoneda"]),
            issuer_name: string_at(&root, &["emisor", "nombre"]),
            issuer_nit: string_at(&root, &["emisor", "nit"]),
            issuer_nrc: string_at(&root, &["emisor", "nrc"]),
            receiver_name: string_at(&root, &["receptor", "nombre"]),
            receiver_nit: string_at(&root, &["receptor", "nit"]),
            receiver_nrc: string_at(&root, &["receptor", "nrc"]),
            taxed_sales: money_at(&root, &["resumen", "totalGravada"]),
            exempt_sales: money_at(&root, &["resumen", "totalExenta"]),
            non_subject_sales: money_at(&root, &["resumen", "totalNoSuj"]),
            subtotal_sales: money_at(&root, &["resumen", "subTotalVentas"]),
            discount_total: money_at(&root, &["resumen", "totalDescu"]),
            subtotal: money_at(&root, &["resumen", "subTotal"]),
            taxes: taxes_at(&root),
            income_tax_retention: money_at(&root, &["resumen", "reteRenta"]),
            vat_retention: money_at(&root, &["resumen", "ivaRete1"]),
            vat_perception: money_at(&root, &["resumen", "ivaPerci1"]),
            operation_total: money_at(&root, &["resumen", "montoTotalOperacion"]),
            total_to_pay: money_at(&root, &["resumen", "totalPagar"]),
            total_in_words: string_at(&root, &["resumen", "totalLetras"]),
            payment_condition: payment_condition(number_at(
                &root,
                &["resumen", "condicionOperacion"],
            ))
            .to_string(),
            line_items: line_items_at(&root),
        })
    }

    pub fn file_slug(&self) -> String {
        first_non_empty(&[
            self.control_number.as_str(),
            self.generation_code.as_str(),
            self.source_filename.as_str(),
        ])
        .to_string()
    }
}

impl fmt::Display for InvoiceParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson(error) => write!(formatter, "invalid JSON: {error}"),
            Self::MissingObject => write!(formatter, "invoice JSON root must be an object"),
        }
    }
}

impl std::error::Error for InvoiceParseError {}

fn string_at(root: &Value, path: &[&str]) -> String {
    value_at(root, path)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn number_at(root: &Value, path: &[&str]) -> Option<i64> {
    value_at(root, path).and_then(Value::as_i64)
}

fn money_at(root: &Value, path: &[&str]) -> String {
    value_at(root, path)
        .and_then(Value::as_f64)
        .map(format_money)
        .unwrap_or_else(|| "$0.00".to_string())
}

fn value_at<'a>(root: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(root, |current, key| current.get(key))
}

fn taxes_at(root: &Value) -> Vec<TaxSummary> {
    value_at(root, &["resumen", "tributos"])
        .and_then(Value::as_array)
        .map(|taxes| {
            taxes
                .iter()
                .map(|tax| TaxSummary {
                    code: string_at(tax, &["codigo"]),
                    description: string_at(tax, &["descripcion"]),
                    value: money_at(tax, &["valor"]),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn line_items_at(root: &Value) -> Vec<InvoiceLineItem> {
    value_at(root, &["cuerpoDocumento"])
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .take(5)
                .map(|item| InvoiceLineItem {
                    description: string_at(item, &["descripcion"]),
                    quantity: number_string_at(item, &["cantidad"]),
                    unit_price: money_at(item, &["precioUni"]),
                    taxed_sale: money_at(item, &["ventaGravada"]),
                    exempt_sale: money_at(item, &["ventaExenta"]),
                    non_subject_sale: money_at(item, &["ventaNoSuj"]),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn number_string_at(root: &Value, path: &[&str]) -> String {
    value_at(root, path)
        .and_then(Value::as_f64)
        .map(|number| {
            if number.fract() == 0.0 {
                format!("{number:.0}")
            } else {
                format!("{number:.4}")
            }
        })
        .unwrap_or_default()
}

fn format_money(amount: f64) -> String {
    format!("${amount:.2}")
}

fn document_type_name(code: &str) -> &str {
    match code {
        "03" => "Comprobante de credito fiscal",
        "01" => "Factura consumidor final",
        "05" => "Nota de credito",
        "06" => "Nota de debito",
        "14" => "Factura sujeto excluido",
        _ => "Documento tributario",
    }
}

fn payment_condition(value: Option<i64>) -> &'static str {
    match value {
        Some(1) => "Contado",
        Some(2) => "Credito",
        Some(3) => "Otro",
        _ => "",
    }
}

fn first_non_empty<'a>(values: &[&'a str]) -> &'a str {
    values
        .iter()
        .copied()
        .find(|value| !value.trim().is_empty())
        .unwrap_or("invoice")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ccf_summary_fields() {
        let json = br#"{
            "identificacion": {
                "tipoDte": "03",
                "numeroControl": "DTE-03-ABC12345-000000000000001",
                "codigoGeneracion": "AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE",
                "fecEmi": "2026-05-01",
                "horEmi": "09:30:00",
                "tipoMoneda": "USD"
            },
            "emisor": {"nombre": "Vendor SA", "nit": "06142809101012", "nrc": "123456"},
            "receptor": {"nombre": "My Company", "nit": "06142809101013", "nrc": "654321"},
            "cuerpoDocumento": [
                {"descripcion": "Servicio", "cantidad": 1, "precioUni": 100, "ventaGravada": 100}
            ],
            "resumen": {
                "totalGravada": 100,
                "totalExenta": 0,
                "totalNoSuj": 0,
                "subTotalVentas": 100,
                "totalDescu": 0,
                "subTotal": 100,
                "tributos": [{"codigo": "20", "descripcion": "IVA", "valor": 13}],
                "montoTotalOperacion": 113,
                "totalPagar": 113,
                "totalLetras": "CIENTO TRECE 00/100",
                "condicionOperacion": 1
            }
        }"#;

        let summary = InvoiceSummary::from_json_bytes("invoice.json", json).unwrap();

        assert!(summary.is_ccf);
        assert_eq!(summary.document_type_code, "03");
        assert_eq!(summary.issuer_name, "Vendor SA");
        assert_eq!(summary.total_to_pay, "$113.00");
        assert_eq!(summary.taxes[0].value, "$13.00");
        assert_eq!(summary.payment_condition, "Contado");
    }
}
