use crate::client::client::Client;
use crate::errors::Error;
use roxmltree::Document;
use serde_json::Value;

/// Client for the Salesforce Metadata API's CRUD-based calls.
///
/// The Metadata API offers two styles: file-based calls (`deploy`/`retrieve`,
/// which take a zip and run asynchronously) and CRUD-based calls, which act on
/// components directly and return synchronously. This client covers the latter.
///
/// It exists because some operations have no equivalent elsewhere — notably
/// deleting a `CustomField`, which the Tooling API does not support at all
/// (that object exposes only Query/GET/POST/PATCH).
///
/// Unlike the REST clients this speaks SOAP, since the Metadata API has no REST
/// binding for these calls.
#[derive(Default)]
pub struct MetadataApi {
    pub(crate) client: Client,
}

/// Outcome of one component in a CRUD-based Metadata API call.
#[derive(Debug, Clone, PartialEq)]
pub struct MetadataResult {
    pub full_name: String,
    pub success: bool,
    pub errors: Vec<MetadataError>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetadataError {
    pub message: String,
    pub status_code: String,
    pub fields: Vec<String>,
}

impl MetadataResult {
    /// Joined error text, for surfacing a failure in one line.
    pub fn error_text(&self) -> String {
        self.errors
            .iter()
            .map(|e| {
                if e.status_code.is_empty() {
                    e.message.clone()
                } else {
                    format!("{}: {}", e.status_code, e.message)
                }
            })
            .collect::<Vec<_>>()
            .join("; ")
    }
}

/// Salesforce caps CRUD-based calls at 10 components per request for most
/// types. `CustomMetadata` and `CustomApplication` are the documented
/// exceptions, allowing 200.
pub const MAX_COMPONENTS_PER_CALL: usize = 10;
pub const MAX_COMPONENTS_BULK_TYPES: usize = 200;

/// Components allowed in one CRUD-based call for a given metadata type.
pub fn max_components_for(metadata_type: &str) -> usize {
    match metadata_type {
        "CustomMetadata" | "CustomApplication" => MAX_COMPONENTS_BULK_TYPES,
        _ => MAX_COMPONENTS_PER_CALL,
    }
}

const METADATA_NAMESPACE: &str = "http://soap.sforce.com/2006/04/metadata";
const ENVELOPE_NAMESPACE: &str = "http://schemas.xmlsoap.org/soap/envelope/";

impl MetadataApi {
    pub fn new(client: Client) -> Self {
        MetadataApi { client }
    }

    /// The SOAP metadata endpoint: `{instance_url}/services/Soap/m/{version}`.
    ///
    /// The REST clients carry a `v`-prefixed version (`v60.0`); the SOAP path
    /// wants the bare number.
    fn endpoint(&self) -> Result<String, Error> {
        let instance_url = self.client.instance_url().ok_or(Error::NotLoggedIn)?;
        let version = self.client.version().trim_start_matches('v');
        Ok(format!("{}/services/Soap/m/{}", instance_url, version))
    }

    fn session_id(&self) -> Result<&str, Error> {
        self.client.access_token_value().ok_or(Error::NotLoggedIn)
    }

    /// Delete metadata components by type and full name, synchronously.
    ///
    /// `full_names` are type-specific: a custom field is `Object.Field__c`, a
    /// custom object is `MyObject__c`. At most [`MAX_COMPONENTS_PER_CALL`] may
    /// be passed — see [`delete_metadata_chunked`](Self::delete_metadata_chunked)
    /// to spread a larger set over several calls.
    pub async fn delete_metadata(
        &mut self,
        metadata_type: &str,
        full_names: &[String],
    ) -> Result<Vec<MetadataResult>, Error> {
        if full_names.is_empty() {
            return Ok(Vec::new());
        }
        let limit = max_components_for(metadata_type);
        if full_names.len() > limit {
            return Err(Error::ConfigError(format!(
                "deleteMetadata accepts at most {} {} components per call, got {}",
                limit,
                metadata_type,
                full_names.len()
            )));
        }

        let names = full_names
            .iter()
            .map(|name| format!("<fullNames>{}</fullNames>", escape_xml(name)))
            .collect::<String>();

        let body = format!(
            "<deleteMetadata><type>{}</type>{}</deleteMetadata>",
            escape_xml(metadata_type),
            names
        );

        let response = self.send("deleteMetadata", &body).await?;
        parse_metadata_results(&response)
    }

    /// Delete any number of components, splitting into per-call batches.
    ///
    /// Results come back in request order. A failing batch aborts the rest —
    /// callers wanting partial progress should batch themselves.
    pub async fn delete_metadata_chunked(
        &mut self,
        metadata_type: &str,
        full_names: &[String],
    ) -> Result<Vec<MetadataResult>, Error> {
        let mut results = Vec::with_capacity(full_names.len());
        for chunk in full_names.chunks(max_components_for(metadata_type)) {
            results.extend(self.delete_metadata(metadata_type, chunk).await?);
        }
        Ok(results)
    }

    /// Create metadata components, synchronously.
    ///
    /// Each component is a JSON object carrying `fullName` plus the type's own
    /// fields, mirroring the Tooling API's `Metadata` shape. Use this over the
    /// Tooling API when a type has fields Tooling's JSON mapping cannot express
    /// — `EncryptedText`'s `maskChar`, for instance, which Tooling rejects as a
    /// complexvalue.
    pub async fn create_metadata(
        &mut self,
        metadata_type: &str,
        components: &[Value],
    ) -> Result<Vec<MetadataResult>, Error> {
        if components.is_empty() {
            return Ok(Vec::new());
        }
        let limit = max_components_for(metadata_type);
        if components.len() > limit {
            return Err(Error::ConfigError(format!(
                "createMetadata accepts at most {} {} components per call, got {}",
                limit,
                metadata_type,
                components.len()
            )));
        }

        let metadata = components
            .iter()
            .map(|component| {
                format!(
                    r#"<metadata xsi:type="{}">{}</metadata>"#,
                    escape_xml(metadata_type),
                    value_to_xml(component)
                )
            })
            .collect::<String>();

        let response = self
            .send(
                "createMetadata",
                &format!("<createMetadata>{}</createMetadata>", metadata),
            )
            .await?;
        parse_metadata_results(&response)
    }

    /// Update metadata components, synchronously.
    ///
    /// The update *replaces* each component, so a partial payload silently
    /// drops whatever it omits — read the current definition first and send it
    /// back with your changes applied.
    pub async fn update_metadata(
        &mut self,
        metadata_type: &str,
        components: &[Value],
    ) -> Result<Vec<MetadataResult>, Error> {
        if components.is_empty() {
            return Ok(Vec::new());
        }
        let limit = max_components_for(metadata_type);
        if components.len() > limit {
            return Err(Error::ConfigError(format!(
                "updateMetadata accepts at most {} {} components per call, got {}",
                limit,
                metadata_type,
                components.len()
            )));
        }

        let metadata = components
            .iter()
            .map(|component| {
                format!(
                    r#"<metadata xsi:type="{}">{}</metadata>"#,
                    escape_xml(metadata_type),
                    value_to_xml(component)
                )
            })
            .collect::<String>();

        let response = self
            .send(
                "updateMetadata",
                &format!("<updateMetadata>{}</updateMetadata>", metadata),
            )
            .await?;
        parse_metadata_results(&response)
    }

    /// Wrap a call body in a SOAP envelope and post it to the metadata endpoint.
    async fn send(&mut self, action: &str, body: &str) -> Result<String, Error> {
        let envelope = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<soapenv:Envelope xmlns:soapenv="{}" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns="{}">
  <soapenv:Header><SessionHeader><sessionId>{}</sessionId></SessionHeader></soapenv:Header>
  <soapenv:Body>{}</soapenv:Body>
</soapenv:Envelope>"#,
            ENVELOPE_NAMESPACE,
            METADATA_NAMESPACE,
            escape_xml(self.session_id()?),
            body
        );

        let url = self.endpoint()?;
        let headers = vec![
            ("Content-Type".to_string(), "text/xml; charset=UTF-8".to_string()),
            ("SOAPAction".to_string(), format!("\"{}\"", action)),
            ("Accept".to_string(), "text/xml".to_string()),
        ];

        let response = self
            .client
            .post_raw_buffer(url, envelope.into_bytes(), headers)
            .await?;

        let status = response.status();
        let text = response.text().await?;

        // SOAP reports faults with a 500, and the fault string is the useful part.
        if !status.is_success() {
            return Err(Error::ConfigError(
                parse_soap_fault(&text)
                    .unwrap_or_else(|| format!("Metadata API returned {}: {}", status, text)),
            ));
        }

        Ok(text)
    }
}

/// Pull `faultstring` out of a SOAP fault body.
fn parse_soap_fault(xml: &str) -> Option<String> {
    let document = Document::parse(xml).ok()?;
    document
        .descendants()
        .find(|n| n.has_tag_name("faultstring"))
        .and_then(|n| n.text())
        .map(str::to_string)
}

/// Read the `result` elements of a CRUD-based metadata response.
fn parse_metadata_results(xml: &str) -> Result<Vec<MetadataResult>, Error> {
    let document = Document::parse(xml)
        .map_err(|e| Error::ConfigError(format!("Could not parse metadata response: {}", e)))?;

    let child_text = |node: roxmltree::Node, name: &str| -> String {
        node.children()
            .find(|c| c.has_tag_name(name))
            .and_then(|c| c.text())
            .unwrap_or_default()
            .to_string()
    };

    let results = document
        .descendants()
        .filter(|n| n.has_tag_name("result"))
        .map(|node| {
            let errors = node
                .children()
                .filter(|c| c.has_tag_name("errors"))
                .map(|error| MetadataError {
                    message: child_text(error, "message"),
                    status_code: child_text(error, "statusCode"),
                    fields: error
                        .children()
                        .filter(|c| c.has_tag_name("fields"))
                        .filter_map(|c| c.text())
                        .map(str::to_string)
                        .collect(),
                })
                .collect();

            MetadataResult {
                full_name: child_text(node, "fullName"),
                success: child_text(node, "success") == "true",
                errors,
            }
        })
        .collect();

    Ok(results)
}

/// Serialise a JSON object into Metadata API XML elements.
///
/// The WSDL declares each type's fields as an ordered sequence, and Salesforce
/// rejects elements out of order. That sequence is alphabetical, with
/// `fullName` first, so keys are emitted that way rather than in JSON order.
fn value_to_xml(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_by(|a, b| match (a.as_str(), b.as_str()) {
                ("fullName", "fullName") => std::cmp::Ordering::Equal,
                ("fullName", _) => std::cmp::Ordering::Less,
                (_, "fullName") => std::cmp::Ordering::Greater,
                (left, right) => left.cmp(right),
            });

            keys.into_iter()
                .map(|key| {
                    let child = &map[key];
                    match child {
                        // An array repeats the element once per entry.
                        Value::Array(items) => items
                            .iter()
                            .map(|item| element(key, &value_to_xml(item)))
                            .collect::<String>(),
                        // Null means "absent", not an empty element.
                        Value::Null => String::new(),
                        _ => element(key, &value_to_xml(child)),
                    }
                })
                .collect()
        }
        Value::String(text) => escape_xml(text),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn element(name: &str, inner: &str) -> String {
    format!("<{name}>{inner}</{name}>", name = escape_xml(name), inner = inner)
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_successful_delete() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/">
  <soapenv:Body>
    <deleteMetadataResponse>
      <result><fullName>Account.Dev_Text__c</fullName><success>true</success></result>
    </deleteMetadataResponse>
  </soapenv:Body>
</soapenv:Envelope>"#;

        let results = parse_metadata_results(xml).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].full_name, "Account.Dev_Text__c");
        assert!(results[0].success);
        assert!(results[0].errors.is_empty());
    }

    #[test]
    fn parses_mixed_results_in_order() {
        let xml = r#"<soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/">
  <soapenv:Body>
    <deleteMetadataResponse>
      <result><fullName>Account.A__c</fullName><success>true</success></result>
      <result>
        <fullName>Account.B__c</fullName>
        <success>false</success>
        <errors>
          <message>Cannot find the field</message>
          <statusCode>INVALID_CROSS_REFERENCE_KEY</statusCode>
          <fields>B__c</fields>
        </errors>
      </result>
    </deleteMetadataResponse>
  </soapenv:Body>
</soapenv:Envelope>"#;

        let results = parse_metadata_results(xml).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].full_name, "Account.A__c");
        assert!(results[0].success);

        assert_eq!(results[1].full_name, "Account.B__c");
        assert!(!results[1].success);
        assert_eq!(results[1].errors.len(), 1);
        assert_eq!(results[1].errors[0].status_code, "INVALID_CROSS_REFERENCE_KEY");
        assert_eq!(results[1].errors[0].fields, vec!["B__c".to_string()]);
        assert_eq!(
            results[1].error_text(),
            "INVALID_CROSS_REFERENCE_KEY: Cannot find the field"
        );
    }

    #[test]
    fn collects_several_errors_for_one_component() {
        let xml = r#"<Envelope><Body><deleteMetadataResponse>
      <result>
        <fullName>Account.A__c</fullName>
        <success>false</success>
        <errors><message>first</message><statusCode>ONE</statusCode></errors>
        <errors><message>second</message><statusCode>TWO</statusCode></errors>
      </result>
    </deleteMetadataResponse></Body></Envelope>"#;

        let results = parse_metadata_results(xml).unwrap();
        assert_eq!(results[0].errors.len(), 2);
        assert_eq!(results[0].error_text(), "ONE: first; TWO: second");
    }

    #[test]
    fn error_text_without_a_status_code_is_just_the_message() {
        let result = MetadataResult {
            full_name: "Account.A__c".to_string(),
            success: false,
            errors: vec![MetadataError {
                message: "something went wrong".to_string(),
                status_code: String::new(),
                fields: Vec::new(),
            }],
        };
        assert_eq!(result.error_text(), "something went wrong");
    }

    #[test]
    fn an_empty_response_yields_no_results() {
        let xml = r#"<Envelope><Body><deleteMetadataResponse/></Body></Envelope>"#;
        assert!(parse_metadata_results(xml).unwrap().is_empty());
    }

    #[test]
    fn reads_a_soap_fault_string() {
        let xml = r#"<soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/">
  <soapenv:Body>
    <soapenv:Fault>
      <faultcode>sf:INVALID_SESSION_ID</faultcode>
      <faultstring>INVALID_SESSION_ID: Session expired or invalid</faultstring>
    </soapenv:Fault>
  </soapenv:Body>
</soapenv:Envelope>"#;

        assert_eq!(
            parse_soap_fault(xml).unwrap(),
            "INVALID_SESSION_ID: Session expired or invalid"
        );
    }

    #[test]
    fn a_non_fault_body_has_no_fault_string() {
        assert!(parse_soap_fault("<Envelope><Body><ok/></Body></Envelope>").is_none());
    }

    #[test]
    fn xml_special_characters_are_escaped() {
        assert_eq!(escape_xml("a&b"), "a&amp;b");
        assert_eq!(escape_xml("<tag>"), "&lt;tag&gt;");
        assert_eq!(escape_xml("it's \"quoted\""), "it&apos;s &quot;quoted&quot;");
    }

    #[test]
    fn full_name_leads_and_the_rest_is_alphabetical() {
        // The WSDL declares an ordered sequence and Salesforce rejects elements
        // out of order, so JSON insertion order must not leak through.
        let xml = value_to_xml(&serde_json::json!({
            "label": "Dev",
            "type": "Text",
            "fullName": "Account.Dev__c",
            "length": 100,
        }));
        assert_eq!(
            xml,
            "<fullName>Account.Dev__c</fullName><label>Dev</label><length>100</length><type>Text</type>"
        );
    }

    #[test]
    fn booleans_and_numbers_render_without_quotes() {
        let xml = value_to_xml(&serde_json::json!({ "required": false, "scale": 2 }));
        assert_eq!(xml, "<required>false</required><scale>2</scale>");
    }

    #[test]
    fn nested_objects_nest_their_elements() {
        let xml = value_to_xml(&serde_json::json!({
            "valueSet": { "restricted": true, "valueSetDefinition": { "sorted": false } }
        }));
        assert_eq!(
            xml,
            "<valueSet><restricted>true</restricted><valueSetDefinition><sorted>false</sorted></valueSetDefinition></valueSet>"
        );
    }

    #[test]
    fn arrays_repeat_their_element() {
        let xml = value_to_xml(&serde_json::json!({
            "value": [{ "fullName": "A" }, { "fullName": "B" }]
        }));
        assert_eq!(
            xml,
            "<value><fullName>A</fullName></value><value><fullName>B</fullName></value>"
        );
    }

    #[test]
    fn nulls_are_omitted_rather_than_sent_empty() {
        let xml = value_to_xml(&serde_json::json!({ "description": null, "label": "Dev" }));
        assert_eq!(xml, "<label>Dev</label>");
    }

    #[test]
    fn text_content_is_escaped() {
        let xml = value_to_xml(&serde_json::json!({ "label": "R&D <test>" }));
        assert_eq!(xml, "<label>R&amp;D &lt;test&gt;</label>");
    }

    #[test]
    fn encrypted_text_mask_settings_serialise_as_plain_elements() {
        // The exact payload the Tooling API rejected as a complexvalue.
        let xml = value_to_xml(&serde_json::json!({
            "fullName": "Account.Dev_EncryptedText__c",
            "type": "EncryptedText",
            "maskChar": "asterisk",
            "maskType": "all",
            "length": 100,
        }));
        assert!(xml.contains("<maskChar>asterisk</maskChar>"), "{}", xml);
        assert!(xml.contains("<maskType>all</maskType>"), "{}", xml);
    }

    #[test]
    fn bulk_types_get_the_larger_component_limit() {
        assert_eq!(max_components_for("CustomField"), 10);
        assert_eq!(max_components_for("CustomMetadata"), 200);
        assert_eq!(max_components_for("CustomApplication"), 200);
    }

    #[tokio::test]
    async fn updating_nothing_makes_no_call() {
        let mut api = MetadataApi::new(Client::new());
        assert!(api.update_metadata("CustomField", &[]).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn oversized_updates_are_rejected_before_sending() {
        let mut api = MetadataApi::new(Client::new());
        let components: Vec<serde_json::Value> = (0..11)
            .map(|i| serde_json::json!({ "fullName": format!("Account.F{}__c", i) }))
            .collect();
        let error = api.update_metadata("CustomField", &components).await.unwrap_err();
        assert!(format!("{}", error).contains("at most 10"), "{}", error);
    }

    #[tokio::test]
    async fn creating_nothing_makes_no_call() {
        let mut api = MetadataApi::new(Client::new());
        assert!(api.create_metadata("CustomField", &[]).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn deleting_nothing_makes_no_call() {
        // No client configured — this would fail at the request if one was made.
        let mut api = MetadataApi::new(Client::new());
        assert!(api.delete_metadata("CustomField", &[]).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn oversized_batches_are_rejected_before_sending() {
        let mut api = MetadataApi::new(Client::new());
        let names: Vec<String> = (0..11).map(|i| format!("Account.F{}__c", i)).collect();
        let error = api.delete_metadata("CustomField", &names).await.unwrap_err();
        assert!(format!("{}", error).contains("at most 10"), "{}", error);
    }
}
