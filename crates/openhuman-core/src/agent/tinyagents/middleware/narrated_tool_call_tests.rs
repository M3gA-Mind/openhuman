use super::*;
use serde_json::json;
use tinyagents_harness::context::RunConfig;

// Each fixture is a narrated call copied from a real prod transcript (#6344).

#[test]
fn gemma_call_with_its_own_string_delimiter() {
    let recovered =
        recover(r#"<|tool_call>call:NOTION_FETCH_DATA{fetch_type:<|"|>pages<|"|>}<tool_call|>"#);
    assert_eq!(
        recovered.calls,
        vec![NarratedCall {
            name: "NOTION_FETCH_DATA".into(),
            arguments: json!({"fetch_type": "pages"}),
        }]
    );
    assert_eq!(recovered.prose, "");
}

#[test]
fn gemma_call_with_plain_quotes_numbers_and_an_empty_string() {
    let recovered = recover(
        r#"<|tool_call>call:NOTION_SEARCH_NOTION_PAGE{filter_value:"page",page_size:100,query:""}<tool_call|>"#,
    );
    assert_eq!(recovered.calls[0].name, "NOTION_SEARCH_NOTION_PAGE");
    assert_eq!(
        recovered.calls[0].arguments,
        json!({"filter_value": "page", "page_size": 100, "query": ""})
    );
}

#[test]
fn gemma_call_in_its_json_form() {
    let recovered = recover(
        r#"<|tool_call>call:{"name": "GOOGLECALENDAR_EVENTS_LIST", "arguments": {"calendarId": "primary", "singleEvents": true}}<tool_call|>"#,
    );
    assert_eq!(recovered.calls[0].name, "GOOGLECALENDAR_EVENTS_LIST");
    assert_eq!(
        recovered.calls[0].arguments,
        json!({"calendarId": "primary", "singleEvents": true})
    );
}

#[test]
fn a_bare_key_inside_a_string_value_is_not_quoted() {
    let recovered = recover(r#"<|tool_call>call:shell{command:"echo a:b, c:d"}<tool_call|>"#);
    assert_eq!(
        recovered.calls[0].arguments,
        json!({"command": "echo a:b, c:d"})
    );
}

#[test]
fn dsml_invoke_with_a_string_parameter() {
    let recovered = recover(
        "<｜DSML｜tool_calls>\n<｜DSML｜invoke name=\"shell\">\n<｜DSML｜parameter name=\"command\" string=\"true\">echo \"checking for notion result\"</｜DSML｜parameter>\n</｜DSML｜invoke>\n</｜DSML｜tool_calls>",
    );
    assert_eq!(
        recovered.calls,
        vec![NarratedCall {
            name: "shell".into(),
            arguments: json!({"command": "echo \"checking for notion result\""}),
        }]
    );
    assert_eq!(recovered.prose, "");
}

#[test]
fn dsml_non_string_parameter_is_decoded_as_json() {
    let recovered = recover(
        "<｜DSML｜tool_calls><｜DSML｜invoke name=\"validate_workflow\"><｜DSML｜parameter name=\"graph\" string=\"false\">{\"nodes\": [], \"edges\": []}</｜DSML｜parameter></｜DSML｜invoke></｜DSML｜tool_calls>",
    );
    assert_eq!(
        recovered.calls[0].arguments,
        json!({"graph": {"nodes": [], "edges": []}})
    );
}

#[test]
fn prose_around_a_call_is_kept() {
    let recovered = recover(
        "Let me fetch that.\n<|tool_call>call:NOTION_FETCH_DATA{fetch_type:\"pages\"}<tool_call|>",
    );
    assert_eq!(recovered.prose, "Let me fetch that.");
    assert_eq!(recovered.calls.len(), 1);
}

#[test]
fn ordinary_text_and_the_xml_form_are_left_alone() {
    for text in [
        "No tools needed here.",
        r#"Call it like <tool_call>{"name":"x","arguments":{}}</tool_call>."#,
    ] {
        let recovered = recover(text);
        assert!(recovered.calls.is_empty(), "{text}");
        assert_eq!(recovered.prose, text);
        assert!(!recovered.has_unparsed_markup());
    }
}

#[test]
fn a_cut_off_call_is_reported_as_unparsed() {
    let recovered = recover("<|tool_call>call:NOTION_FETCH_DATA{fetch_type:\"pa");
    assert!(recovered.calls.is_empty());
    assert!(recovered.has_unparsed_markup());
}

fn ctx() -> RunContext<()> {
    RunContext::new(RunConfig::new("narrated-test"), ())
}

#[tokio::test]
async fn a_narrated_call_becomes_a_structured_call_the_loop_runs() {
    let mut response = ModelResponse::assistant(
        "<｜DSML｜tool_calls><｜DSML｜invoke name=\"shell\"><｜DSML｜parameter name=\"command\" string=\"true\">ls</｜DSML｜parameter></｜DSML｜invoke></｜DSML｜tool_calls>",
    );
    NarratedToolCallMiddleware
        .after_model(&mut ctx(), &(), &mut response)
        .await
        .expect("a parseable narrated call is recovered, not refused");

    let calls = response.tool_calls();
    assert_eq!(
        calls.len(),
        1,
        "the narrated call must reach the loop as a tool call"
    );
    assert_eq!(calls[0].name, "shell");
    assert_eq!(calls[0].arguments, json!({"command": "ls"}));
    assert!(
        response
            .message
            .content
            .iter()
            .all(|b| b.as_text().is_none()),
        "no markup may remain as message text"
    );
    assert_eq!(response.finish_reason.as_deref(), Some("tool_calls"));
}

#[tokio::test]
async fn unparseable_markup_fails_the_run_instead_of_becoming_the_answer() {
    let mut response = ModelResponse::assistant("<|tool_call>call:NOTION_FETCH_DATA{fetch_type:");
    let error = NarratedToolCallMiddleware
        .after_model(&mut ctx(), &(), &mut response)
        .await
        .expect_err("markup that cannot run must not be returned as content");
    assert!(error.to_string().contains("could not be parsed"), "{error}");
}

#[tokio::test]
async fn a_response_with_structured_calls_is_untouched() {
    let mut response = ModelResponse::assistant("<|tool_call>call:x{}<tool_call|>");
    response.message.tool_calls.push(TaToolCall {
        id: "c1".into(),
        name: "real".into(),
        arguments: json!({}),
        invalid: None,
    });
    NarratedToolCallMiddleware
        .after_model(&mut ctx(), &(), &mut response)
        .await
        .unwrap();
    assert_eq!(response.tool_calls().len(), 1);
    assert_eq!(response.tool_calls()[0].name, "real");
}
