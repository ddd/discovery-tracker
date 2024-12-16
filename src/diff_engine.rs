use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::parser::{DiscoveryDocument, Schema, Resource, Method};
use crate::parser::Property;

#[derive(Debug)]
pub struct DiffEngine;

#[derive(Debug, Serialize, Deserialize)]
pub struct Change {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_value: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChangeSet {
    pub service: String,
    pub modifications: Vec<Change>,
    pub additions: Vec<Change>,
    pub deletions: Vec<Change>,
}

impl DiffEngine {
    pub fn new() -> Self {
        DiffEngine
    }

    pub fn diff(&self, old: &DiscoveryDocument, new: &DiscoveryDocument, service: &str) -> ChangeSet {
        let mut modifications = Vec::new();
        let mut additions = Vec::new();
        let mut deletions = Vec::new();

        self.compare_top_level(old, new, &mut modifications, &mut additions, &mut deletions);
        self.compare_schemas(&old.schemas, &new.schemas, &mut modifications, &mut additions, &mut deletions);
        self.compare_resources(&old.resources, &new.resources, &mut modifications, &mut additions, &mut deletions);

        ChangeSet {
            service: service.to_string(),
            modifications,
            additions,
            deletions,
        }
    }

    fn compare_top_level(&self, old: &DiscoveryDocument, new: &DiscoveryDocument, 
                         modifications: &mut Vec<Change>, 
                         additions: &mut Vec<Change>, 
                         deletions: &mut Vec<Change>) {
        self.compare_field("description", &old.description, &new.description, modifications, additions, deletions);
        self.compare_field("title", &old.title, &new.title, modifications, additions, deletions);
        self.compare_field("discoveryVersion", &old.discovery_version, &new.discovery_version, modifications, additions, deletions);
        self.compare_field("revision", &old.revision, &new.revision, modifications, additions, deletions);
        self.compare_field("ownerDomain", &old.owner_domain, &new.owner_domain, modifications, additions, deletions);
        self.compare_field("baseUrl", &old.base_url, &new.base_url, modifications, additions, deletions);
        self.compare_field("documentationLink", &old.documentation_link, &new.documentation_link, modifications, additions, deletions);
    }


    fn compare_schemas(&self, old: &Option<HashMap<String, Schema>>, new: &Option<HashMap<String, Schema>>, 
        modifications: &mut Vec<Change>, 
        additions: &mut Vec<Change>, 
        deletions: &mut Vec<Change>) {
            match (old, new) {
            (Some(old_schemas), Some(new_schemas)) => {
                for (key, new_schema) in new_schemas {
                    match old_schemas.get(key) {
                        Some(old_schema) => self.compare_schema(key, old_schema, new_schema, modifications, additions, deletions),
                        None => additions.push(Change {
                            path: format!("/schemas/{}", key),
                            value: Some(serde_json::to_value(new_schema).unwrap()),
                            old_value: None,
                            new_value: None,
                        }),
                    }
                }
                for (key, old_schema) in old_schemas {
                    if !new_schemas.contains_key(key) {
                        deletions.push(Change {
                            path: format!("/schemas/{}", key),
                            value: None,
                            old_value: Some(serde_json::to_value(old_schema).unwrap()),
                            new_value: None,
                        });
                    }
                }
            }
            (None, Some(new_schemas)) => additions.push(Change {
                    path: "/schemas".to_string(),
                    value: Some(serde_json::to_value(new_schemas).unwrap()),
                    old_value: None,
                    new_value: None,
                }),
            (Some(old_schemas), None) => deletions.push(Change {
                    path: "/schemas".to_string(),
                    value: None,
                    old_value: Some(serde_json::to_value(old_schemas).unwrap()),
                    new_value: None,
                }),
            (None, None) => {}
            }
    }

    fn compare_schema(&self, key: &str, old: &Schema, new: &Schema, 
                      modifications: &mut Vec<Change>, 
                      additions: &mut Vec<Change>, 
                      deletions: &mut Vec<Change>) {
        let path = format!("/schemas/{}", key);
        match (old, new) {
            (Schema::Object(old_obj), Schema::Object(new_obj)) => {
                self.compare_field(&format!("{}/type", path), &old_obj.schema_type, &new_obj.schema_type, modifications, additions, deletions);
                self.compare_field(&format!("{}/id", path), &old_obj.id, &new_obj.id, modifications, additions, deletions);
                self.compare_properties(&path, &old_obj.properties, &new_obj.properties, modifications, additions, deletions);
            }
            (Schema::Enum(old_enum), Schema::Enum(new_enum)) => {
                self.compare_field(&format!("{}/type", path), &old_enum.schema_type, &new_enum.schema_type, modifications, additions, deletions);
                self.compare_field(&format!("{}/id", path), &old_enum.id, &new_enum.id, modifications, additions, deletions);
                self.compare_properties(&path, &old_enum.properties, &new_enum.properties, modifications, additions, deletions);
                self.compare_field(&format!("{}/enumeration", path), &Some(old_enum.enumeration.clone()), &Some(new_enum.enumeration.clone()), modifications, additions, deletions);
                self.compare_field(&format!("{}/enumDescriptions", path), &old_enum.enum_descriptions, &new_enum.enum_descriptions, modifications, additions, deletions);
            }
            _ => modifications.push(Change {
                path,
                value: None,
                old_value: Some(serde_json::to_value(old).unwrap()),
                new_value: Some(serde_json::to_value(new).unwrap()),
            }),
        }
    }

    fn compare_properties(&self, path: &str, old: &Option<HashMap<String, Property>>, new: &Option<HashMap<String, Property>>, 
                          modifications: &mut Vec<Change>, 
                          additions: &mut Vec<Change>, 
                          deletions: &mut Vec<Change>) {
        match (old, new) {
            (Some(old_props), Some(new_props)) => {
                for (key, new_prop) in new_props {
                    let prop_path = format!("{}/properties/{}", path, key);
                    match old_props.get(key) {
                        Some(old_prop) => {
                            // Compare type
                            self.compare_field(&format!("{}/type", prop_path), &old_prop.property_type, &new_prop.property_type, modifications, additions, deletions);
                            // Compare reference
                            self.compare_field(&format!("{}/$ref", prop_path), &old_prop.reference, &new_prop.reference, modifications, additions, deletions);
                            // Compare format
                            self.compare_field(&format!("{}/format", prop_path), &old_prop.format, &new_prop.format, modifications, additions, deletions);
                            // Compare description
                            self.compare_field(&format!("{}/description", prop_path), &old_prop.description, &new_prop.description, modifications, additions, deletions);
                        }
                        None => additions.push(Change {
                            path: prop_path,
                            value: Some(serde_json::to_value(new_prop).unwrap()),
                            old_value: None,
                            new_value: None,
                        }),
                    }
                }
                for (key, old_prop) in old_props {
                    if !new_props.contains_key(key) {
                        let prop_path = format!("{}/properties/{}", path, key);
                        // For complete property deletion, include the full property data
                        deletions.push(Change {
                            path: prop_path.clone(),
                            value: None,
                            old_value: Some(serde_json::to_value(old_prop).unwrap()),
                            new_value: None,
                        });
                    }
                }
            }
            (None, Some(new_props)) => additions.push(Change {
                path: format!("{}/properties", path),
                value: Some(serde_json::to_value(new_props).unwrap()),
                old_value: None,
                new_value: None,
            }),
            (Some(old_props), None) => deletions.push(Change {
                path: format!("{}/properties", path),
                value: None,
                old_value: Some(serde_json::to_value(old_props).unwrap()),
                new_value: None,
            }),
            (None, None) => {}
        }
    }


    fn compare_resources(&self, old: &Option<HashMap<String, Resource>>, new: &Option<HashMap<String, Resource>>, 
                         modifications: &mut Vec<Change>, 
                         additions: &mut Vec<Change>, 
                         deletions: &mut Vec<Change>) {
        match (old, new) {
            (Some(old_resources), Some(new_resources)) => {
                for (key, new_resource) in new_resources {
                    let resource_path = format!("/resources/{}", key);
                    match old_resources.get(key) {
                        Some(old_resource) => self.compare_methods(&resource_path, &old_resource.methods, &new_resource.methods, modifications, additions, deletions),
                        None => additions.push(Change {
                            path: resource_path,
                            value: Some(serde_json::to_value(new_resource).unwrap()),
                            old_value: None,
                            new_value: None,
                        }),
                    }
                }
                for key in old_resources.keys() {
                    if !new_resources.contains_key(key) {
                        deletions.push(Change {
                            path: format!("/resources/{}", key),
                            value: None,
                            old_value: None,
                            new_value: None,
                        });
                    }
                }
            }
            (None, Some(new_resources)) => additions.push(Change {
                path: "/resources".to_string(),
                value: Some(serde_json::to_value(new_resources).unwrap()),
                old_value: None,
                new_value: None,
            }),
            (Some(_), None) => deletions.push(Change {
                path: "/resources".to_string(),
                value: None,
                old_value: None,
                new_value: None,
            }),
            (None, None) => {}
        }
    }

    fn compare_methods(&self, path: &str, old: &Option<HashMap<String, Method>>, new: &Option<HashMap<String, Method>>, 
                       modifications: &mut Vec<Change>, 
                       additions: &mut Vec<Change>, 
                       deletions: &mut Vec<Change>) {
        match (old, new) {
            (Some(old_methods), Some(new_methods)) => {
                for (key, new_method) in new_methods {
                    let method_path = format!("{}/methods/{}", path, key);
                    match old_methods.get(key) {
                        Some(old_method) => {
                            self.compare_field(&format!("{}/id", method_path), &Some(old_method.id.clone()), &Some(new_method.id.clone()), modifications, additions, deletions);
                            self.compare_field(&format!("{}/path", method_path), &Some(old_method.path.clone()), &Some(new_method.path.clone()), modifications, additions, deletions);
                            self.compare_field(&format!("{}/httpMethod", method_path), &Some(old_method.http_method.clone()), &Some(new_method.http_method.clone()), modifications, additions, deletions);
                            self.compare_field(&format!("{}/description", method_path), &old_method.description, &new_method.description, modifications, additions, deletions);
                            self.compare_parameters(&method_path, &old_method.parameters, &new_method.parameters, modifications, additions, deletions);
                            self.compare_field(&format!("{}/request", method_path), &old_method.request, &new_method.request, modifications, additions, deletions);
                            self.compare_field(&format!("{}/response", method_path), &old_method.response, &new_method.response, modifications, additions, deletions);
                            self.compare_field(&format!("{}/scopes", method_path), &old_method.scopes, &new_method.scopes, modifications, additions, deletions);
                        }
                        None => additions.push(Change {
                            path: method_path,
                            value: Some(serde_json::to_value(new_method).unwrap()),
                            old_value: None,
                            new_value: None,
                        }),
                    }
                }
                for key in old_methods.keys() {
                    if !new_methods.contains_key(key) {
                        deletions.push(Change {
                            path: format!("{}/methods/{}", path, key),
                            value: None,
                            old_value: None,
                            new_value: None,
                        });
                    }
                }
            }
            (None, Some(new_methods)) => additions.push(Change {
                path: format!("{}/methods", path),
                value: Some(serde_json::to_value(new_methods).unwrap()),
                old_value: None,
                new_value: None,
            }),
            (Some(_), None) => deletions.push(Change {
                path: format!("{}/methods", path),
                value: None,
                old_value: None,
                new_value: None,
            }),
            (None, None) => {}
        }
    }

    fn compare_parameters(&self, path: &str, old: &Option<HashMap<String, crate::parser::Parameter>>, new: &Option<HashMap<String, crate::parser::Parameter>>, 
                          modifications: &mut Vec<Change>, 
                          additions: &mut Vec<Change>, 
                          deletions: &mut Vec<Change>) {
        match (old, new) {
            (Some(old_params), Some(new_params)) => {
                for (key, new_param) in new_params {
                    let param_path = format!("{}/parameters/{}", path, key);
                    match old_params.get(key) {
                        Some(old_param) => {
                            self.compare_field(&format!("{}/type", param_path), &old_param.param_type, &new_param.param_type, modifications, additions, deletions);
                            self.compare_field(&format!("{}/description", param_path), &old_param.description, &new_param.description, modifications, additions, deletions);
                            self.compare_field(&format!("{}/required", param_path), &old_param.required, &new_param.required, modifications, additions, deletions);
                            self.compare_field(&format!("{}/location", param_path), &old_param.location, &new_param.location, modifications, additions, deletions);
                        }
                        None => additions.push(Change {
                            path: param_path,
                            value: Some(serde_json::to_value(new_param).unwrap()),
                            old_value: None,
                            new_value: None,
                        }),
                    }
                }
                for key in old_params.keys() {
                    if !new_params.contains_key(key) {
                        deletions.push(Change {
                            path: format!("{}/parameters/{}", path, key),
                            value: None,
                            old_value: None,
                            new_value: None,
                        });
                    }
                }
            }
            (None, Some(new_params)) => additions.push(Change {
                path: format!("{}/parameters", path),
                value: Some(serde_json::to_value(new_params).unwrap()),
                old_value: None,
                new_value: None,
            }),
            (Some(_), None) => deletions.push(Change {
                path: format!("{}/parameters", path),
                value: None,
                old_value: None,
                new_value: None,
            }),
            (None, None) => {}
        }
    }

    fn compare_field<T: PartialEq + serde::Serialize>(
        &self, 
        path: &str, 
        old: &Option<T>, 
        new: &Option<T>, 
        modifications: &mut Vec<Change>, 
        additions: &mut Vec<Change>, 
        deletions: &mut Vec<Change>
    ) {
        match (old, new) {
            (Some(old_value), Some(new_value)) if old_value != new_value => {
                modifications.push(Change {
                    path: path.to_string(),
                    value: None,
                    old_value: Some(serde_json::to_value(old_value).unwrap()),
                    new_value: Some(serde_json::to_value(new_value).unwrap()),
                });
            }
            (Some(old_value), None) => {
                deletions.push(Change {
                    path: path.to_string(),
                    value: None,
                    old_value: Some(serde_json::to_value(old_value).unwrap()),
                    new_value: None,
                });
            }
            (None, Some(new_value)) => {
                additions.push(Change {
                    path: path.to_string(),
                    value: Some(serde_json::to_value(new_value).unwrap()),
                    old_value: None,
                    new_value: None,
                });
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{DiscoveryDocument, Schema, ObjectSchema, EnumSchema, Property, Resource, Method, Parameter, Request, Response};

    fn create_test_document() -> DiscoveryDocument {
        DiscoveryDocument {
            description: Some("Test API".to_string()),
            title: Some("Test".to_string()),
            discovery_version: Some("v1".to_string()),
            revision: Some("20210101".to_string()),
            owner_domain: Some("example.com".to_string()),
            base_url: Some("https://api.example.com/".to_string()),
            documentation_link: Some("https://docs.example.com/".to_string()),
            schemas: Some(HashMap::new()),
            resources: Some(HashMap::new()),
        }
    }

    #[test]
    fn test_deletion_with_old_value() {
        let mut old_doc = create_test_document();
        let mut new_doc = create_test_document();

        old_doc.base_url = Some("https://api.example.com/".to_string());
        new_doc.base_url = None;

        let diff_engine = DiffEngine::new();
        let change_set = diff_engine.diff(&old_doc, &new_doc, "example.googleapis.com");

        assert_eq!(change_set.modifications.len(), 0);
        assert_eq!(change_set.additions.len(), 0);
        assert_eq!(change_set.deletions.len(), 1);

        let deletion = &change_set.deletions[0];
        assert_eq!(deletion.path, "baseUrl");
        assert_eq!(deletion.old_value, Some(serde_json::json!("https://api.example.com/")));
        assert!(deletion.value.is_none());
        assert!(deletion.new_value.is_none());
    }

    #[test]
    fn test_top_level_changes() {
        let old_doc = create_test_document();
        let mut new_doc = create_test_document();

        new_doc.description = Some("Updated Test API".to_string());
        new_doc.revision = Some("20210102".to_string());
        new_doc.base_url = None;

        let diff_engine = DiffEngine::new();
        let change_set = diff_engine.diff(&old_doc, &new_doc, "example.googleapis.com");

        assert_eq!(change_set.modifications.len(), 2);
        assert_eq!(change_set.additions.len(), 0);
        assert_eq!(change_set.deletions.len(), 1);

        assert!(change_set.modifications.iter().any(|c| c.path == "description"));
        assert!(change_set.modifications.iter().any(|c| c.path == "revision"));
        assert!(change_set.deletions.iter().any(|c| c.path == "baseUrl"));
    }

    #[test]
    fn test_schema_changes() {
        let mut old_doc = create_test_document();
        let mut new_doc = create_test_document();

        let old_schema = Schema::Object(ObjectSchema {
            properties: Some(HashMap::new()),
            schema_type: Some("object".to_string()),
            id: Some("TestObject".to_string()),
        });

        let mut new_schema = Schema::Object(ObjectSchema {
            properties: Some(HashMap::new()),
            schema_type: Some("object".to_string()),
            id: Some("TestObject".to_string()),
        });

        if let Schema::Object(ref mut obj) = new_schema {
            obj.properties.as_mut().unwrap().insert("new_property".to_string(), Property {
                property_type: Some("string".to_string()),
                reference: None,
                format: None,
                description: Some("A new property".to_string()),
            });
        }

        old_doc.schemas.as_mut().unwrap().insert("TestSchema".to_string(), old_schema);
        new_doc.schemas.as_mut().unwrap().insert("TestSchema".to_string(), new_schema);

        let diff_engine = DiffEngine::new();
        let change_set = diff_engine.diff(&old_doc, &new_doc, "example.googleapis.com");

        assert_eq!(change_set.modifications.len(), 0);
        assert_eq!(change_set.additions.len(), 1);
        assert_eq!(change_set.deletions.len(), 0);

        assert!(change_set.additions.iter().any(|c| c.path == "/schemas/TestSchema/properties/new_property"));
    }

    #[test]
    fn test_resource_changes() {
        let mut old_doc = create_test_document();
        let mut new_doc = create_test_document();

        let old_resource = Resource {
            methods: Some(HashMap::new()),
        };

        let mut new_resource = Resource {
            methods: Some(HashMap::new()),
        };

        let new_method = Method {
            id: "test.new".to_string(),
            path: "test/new".to_string(),
            http_method: "POST".to_string(),
            description: Some("A new method".to_string()),
            parameters: Some(HashMap::new()),
            request: Some(Request { reference: Some("TestRequest".to_string()) }),
            response: Some(Response { reference: Some("TestResponse".to_string()) }),
            scopes: Some(vec!["https://www.googleapis.com/auth/test".to_string()]),
        };

        new_resource.methods.as_mut().unwrap().insert("newMethod".to_string(), new_method);

        old_doc.resources.as_mut().unwrap().insert("TestResource".to_string(), old_resource);
        new_doc.resources.as_mut().unwrap().insert("TestResource".to_string(), new_resource);

        let diff_engine = DiffEngine::new();
        let change_set = diff_engine.diff(&old_doc, &new_doc, "example.googleapis.com");

        assert_eq!(change_set.modifications.len(), 0);
        assert_eq!(change_set.additions.len(), 1);
        assert_eq!(change_set.deletions.len(), 0);

        assert!(change_set.additions.iter().any(|c| c.path == "/resources/TestResource/methods/newMethod"));
    }

    #[test]
    fn test_enum_schema_changes() {
        let mut old_doc = create_test_document();
        let mut new_doc = create_test_document();

        let old_schema = Schema::Enum(EnumSchema {
            properties: Some(HashMap::new()),
            schema_type: Some("string".to_string()),
            id: Some("TestEnum".to_string()),
            enumeration: vec!["VALUE1".to_string(), "VALUE2".to_string()],
            enum_descriptions: Some(vec!["Description 1".to_string(), "Description 2".to_string()]),
        });

        let new_schema = Schema::Enum(EnumSchema {
            properties: Some(HashMap::new()),
            schema_type: Some("string".to_string()),
            id: Some("TestEnum".to_string()),
            enumeration: vec!["VALUE1".to_string(), "VALUE2".to_string(), "VALUE3".to_string()],
            enum_descriptions: Some(vec!["Description 1".to_string(), "Updated Description 2".to_string(), "Description 3".to_string()]),
        });

        old_doc.schemas.as_mut().unwrap().insert("TestEnumSchema".to_string(), old_schema);
        new_doc.schemas.as_mut().unwrap().insert("TestEnumSchema".to_string(), new_schema);

        let diff_engine = DiffEngine::new();
        let change_set = diff_engine.diff(&old_doc, &new_doc, "example.googleapis.com");

        assert_eq!(change_set.modifications.len(), 2);
        assert_eq!(change_set.additions.len(), 0);
        assert_eq!(change_set.deletions.len(), 0);

        assert!(change_set.modifications.iter().any(|c| c.path == "/schemas/TestEnumSchema/enumeration"));
        assert!(change_set.modifications.iter().any(|c| c.path == "/schemas/TestEnumSchema/enumDescriptions"));
    }

    #[test]
    fn test_method_parameter_changes() {
        let mut old_doc = create_test_document();
        let mut new_doc = create_test_document();

        let mut old_method = Method {
            id: "test.method".to_string(),
            path: "test/method".to_string(),
            http_method: "GET".to_string(),
            description: Some("Test method".to_string()),
            parameters: Some(HashMap::new()),
            request: None,
            response: Some(Response { reference: Some("TestResponse".to_string()) }),
            scopes: Some(vec!["https://www.googleapis.com/auth/test".to_string()]),
        };

        old_method.parameters.as_mut().unwrap().insert("oldParam".to_string(), Parameter {
            param_type: Some("string".to_string()),
            description: Some("Old parameter".to_string()),
            required: Some(true),
            location: Some("query".to_string()),
        });

        let mut new_method = old_method.clone();
        new_method.parameters.as_mut().unwrap().remove("oldParam");
        new_method.parameters.as_mut().unwrap().insert("newParam".to_string(), Parameter {
            param_type: Some("integer".to_string()),
            description: Some("New parameter".to_string()),
            required: Some(false),
            location: Some("query".to_string()),
        });

        old_doc.resources.as_mut().unwrap().insert("TestResource".to_string(), Resource {
            methods: Some(HashMap::from([("testMethod".to_string(), old_method)])),
        });

        new_doc.resources.as_mut().unwrap().insert("TestResource".to_string(), Resource {
            methods: Some(HashMap::from([("testMethod".to_string(), new_method)])),
        });

        let diff_engine = DiffEngine::new();
        let change_set = diff_engine.diff(&old_doc, &new_doc, "example.googleapis.com");

        assert_eq!(change_set.modifications.len(), 0);
        assert_eq!(change_set.additions.len(), 1);
        assert_eq!(change_set.deletions.len(), 1);

        assert!(change_set.deletions.iter().any(|c| c.path == "/resources/TestResource/methods/testMethod/parameters/oldParam"));
        assert!(change_set.additions.iter().any(|c| c.path == "/resources/TestResource/methods/testMethod/parameters/newParam"));
    }
}