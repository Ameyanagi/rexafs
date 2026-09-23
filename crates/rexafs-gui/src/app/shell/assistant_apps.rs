//! One-time connected-app confirmations for the Codex app-server protocol.
//!
//! The wire shapes are checked against Codex CLI 0.155.1's generated v2 schema.
//! Credential forms and arbitrary questionnaires are declined; the user can
//! complete app setup in Codex. No answer or timeout ever implies consent.
use serde_json::{Value, json};

#[derive(Clone, Debug)]
pub(super) struct AppApproval {
    pub thread: String,
    pub turn: String,
    pub question: String,
    pub details: Vec<String>,
    accept: Value,
    decline: Value,
}

impl AppApproval {
    pub fn is_current(
        &self,
        enabled: bool,
        transcript: &super::Transcript,
        thread: Option<&str>,
    ) -> bool {
        enabled
            && !transcript.stop_pending
            && transcript.accepts(&self.turn)
            && thread == Some(&self.thread)
    }

    pub fn response(&self, id: Value, allow: bool) -> Value {
        json!({"id":id,"result":if allow { &self.accept } else { &self.decline }})
    }
}

/// Always answer unsupported or stale requests so the server cannot wait for
/// a form the host does not display. Empty questionnaire answers cancel input.
pub(super) fn decline_response(request: &Value) -> Value {
    let result = if request["method"] == "mcpServer/elicitation/request" {
        json!({"action":"decline","content":null,"_meta":null})
    } else {
        json!({"answers":{}})
    };
    json!({"id":request["id"],"result":result})
}

pub(super) fn approval(request: &Value) -> Result<AppApproval, String> {
    let p = &request["params"];
    let required = |key: &str| {
        p[key]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| format!("Missing app confirmation {key}"))
    };
    let thread = required("threadId")?;
    let turn = required("turnId")?;
    let (question, details, accept, decline) = match request["method"].as_str() {
        Some("item/tool/requestUserInput" | "tool/requestUserInput") => {
            let questions = p["questions"]
                .as_array()
                .ok_or("Missing app confirmation question")?;
            if questions.len() != 1 {
                return Err("Answer this app's questions in Codex, then try again.".into());
            }
            let q = &questions[0];
            if q["isSecret"] == true || q["isOther"] == true {
                return Err("Complete this app's setup in Codex, then try again.".into());
            }
            let id = q["id"]
                .as_str()
                .filter(|id| !id.is_empty())
                .ok_or("Missing question id")?;
            let question = q["question"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or("Missing question text")?;
            let options = q["options"]
                .as_array()
                .ok_or("Complete this app's setup in Codex, then try again.")?;
            let find = |labels: &[&str]| {
                options.iter().find(|o| {
                    o["label"]
                        .as_str()
                        .is_some_and(|s| labels.iter().any(|label| s.eq_ignore_ascii_case(label)))
                })
            };
            // Recognize only explicit one-time approval options. Never choose
            // an arbitrary first option or a persistent permission grant.
            let yes = find(&["Accept", "Allow once", "Approve once"])
                .ok_or("This app question is not a one-time confirmation. Answer it in Codex.")?;
            let no = find(&["Decline", "Deny", "Cancel"]).ok_or("Missing decline option")?;
            let answer = |option: &Value| json!({"answers":{id:{"answers":[option["label"]]}}});
            let details = yes["description"]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(|s| vec![s.into()])
                .unwrap_or_default();
            (question.into(), details, answer(yes), answer(no))
        }
        Some("mcpServer/elicitation/request") => {
            let schema = &p["requestedSchema"];
            // A confirmation with no fields is representable by Allow/Deny.
            // Do not invent answers for forms, login flows, or verification.
            if !matches!(
                p["mode"].as_str(),
                Some("form" | "openai/form" | "openaiForm")
            ) || schema["type"] != "object"
                || !schema["properties"]
                    .as_object()
                    .is_some_and(|o| o.is_empty())
                || schema
                    .get("required")
                    .is_some_and(|v| !v.as_array().is_some_and(|a| a.is_empty()))
            {
                return Err(
                    "Complete this app's form or connection in Codex, then try again.".into(),
                );
            }
            (
                required("message")?,
                vec![format!("Connected service: {}", required("serverName")?)],
                json!({"action":"accept","content":{},"_meta":null}),
                json!({"action":"decline","content":null,"_meta":null}),
            )
        }
        _ => return Err("Unsupported app confirmation".into()),
    };
    Ok(AppApproval {
        thread,
        turn,
        question,
        details,
        accept,
        decline,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn confirmation() -> Value {
        json!({"id":"approval","method":"item/tool/requestUserInput","params":{
            "threadId":"thread","turnId":"turn","itemId":"tool","questions":[{
                "id":"app-action","header":"Google Drive","question":"Allow this Drive action?",
                "isOther":false,"isSecret":false,"options":[
                    {"label":"Decline","description":"Do not run"},
                    {"label":"Accept","description":"Run this action once"},
                    {"label":"Cancel","description":"Cancel"}]}]}})
    }

    #[test]
    fn answers_explicit_options_and_preserves_question_id() {
        let request = confirmation();
        let parsed = approval(&request).unwrap();
        assert_eq!(
            parsed.response(request["id"].clone(), true)["result"],
            json!({"answers":{"app-action":{"answers":["Accept"]}}})
        );
        assert_eq!(
            parsed.response(request["id"].clone(), false)["result"],
            json!({"answers":{"app-action":{"answers":["Decline"]}}})
        );
        for field in ["turnId", "threadId"] {
            let mut stale = request.clone();
            stale["params"][field] = Value::Null;
            assert!(approval(&stale).is_err());
        }
    }

    #[test]
    fn never_guesses_answers_or_grants_persistent_access() {
        let mut request = confirmation();
        request["params"]["questions"][0]["options"][1]["label"] = json!("Always allow");
        assert!(approval(&request).is_err());
        request = confirmation();
        request["params"]["questions"][0]["isSecret"] = json!(true);
        assert!(approval(&request).is_err());
        assert_eq!(decline_response(&request)["result"], json!({"answers":{}}));
    }

    #[test]
    fn revocation_stop_and_stale_identity_prevent_approval() {
        use super::super::{Event, Transcript};
        let now = std::time::Instant::now();
        let mut transcript = Transcript::default();
        transcript.apply(Event::Send("Read a Drive file".into(), false), now);
        transcript.apply(Event::TurnStarted("turn".into()), now);
        let parsed = approval(&confirmation()).unwrap();
        assert!(parsed.is_current(true, &transcript, Some("thread")));
        assert!(!parsed.is_current(false, &transcript, Some("thread")));
        assert!(!parsed.is_current(true, &transcript, Some("previous-thread")));
        let mut stale = parsed.clone();
        stale.turn = "previous-turn".into();
        assert!(!stale.is_current(true, &transcript, Some("thread")));
        transcript.apply(Event::StopRequested, now);
        assert!(!parsed.is_current(true, &transcript, Some("thread")));
        transcript.apply(Event::Disconnected, now);
        assert!(!parsed.is_current(true, &transcript, Some("thread")));
    }

    #[test]
    fn declines_forms_requiring_data_or_login() {
        let mut request = json!({"id":7,"method":"mcpServer/elicitation/request","params":{
            "threadId":"thread","turnId":"turn","serverName":"codex_apps","mode":"form",
            "message":"Allow this action?","requestedSchema":{"type":"object","properties":{}}}});
        assert_eq!(
            approval(&request).unwrap().response(json!(7), true)["result"]["action"],
            "accept"
        );
        request["params"]["requestedSchema"]["properties"] = json!({"password":{"type":"string"}});
        assert!(approval(&request).is_err());
        request["params"]["mode"] = json!("url");
        assert!(approval(&request).is_err());
        assert_eq!(decline_response(&request)["result"]["action"], "decline");
    }
}
