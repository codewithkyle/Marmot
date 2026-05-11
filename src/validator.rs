#[cfg(test)] mod test;

use crate::parser::{SlotDecl, SlotType, Template};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    DataMustBeObject,
    MissingRequiredSlot {
        name: String,
    },
    WrongType {
        name: String,
        expected: SlotType,
        found: String,
    },
}

pub fn validate_data(template: &Template, data: &Value) -> Result<(), Vec<ValidationError>> {
    let Some(object) = data.as_object() else {
        return Err(vec![ValidationError::DataMustBeObject]);
    };

    let mut errors = Vec::new();

    for slot in &template.slots {
        validate_slot(slot, object.get(&slot.name), &mut errors);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_slot(slot: &SlotDecl, value: Option<&Value>, errors: &mut Vec<ValidationError>) {
    let Some(value) = value else {
        if slot.required {
            errors.push(ValidationError::MissingRequiredSlot {
                name: slot.name.clone(),
            });
        }
        return;
    };

    if !value_matches_slot_type(value, &slot.ty) {
        errors.push(ValidationError::WrongType {
            name: slot.name.clone(),
            expected: slot.ty.clone(),
            found: json_type_name(value).to_string(),
        });
    }
}

fn value_matches_slot_type(value: &Value, ty: &SlotType) -> bool {
    match ty {
        SlotType::String => value.is_string(),
        SlotType::Int => value.as_i64().is_some() || value.as_u64().is_some(),
        SlotType::Decimal => value.is_number(),
    }
}

fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
