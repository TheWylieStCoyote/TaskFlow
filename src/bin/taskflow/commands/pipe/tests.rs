//! Comprehensive tests for the pipe interface.
//!
//! These tests cover:
//! - JSON input parsing (valid/invalid)
//! - Command execution (create, list, get, update, delete)
//! - Output formatting (JSON, YAML, CSV)
//! - Error handling
//! - Request/response roundtrip

#[cfg(test)]
mod pipe_interface_tests {
    use super::super::*;
    use serde_json::json;
    use taskflow::app::Model;
    use taskflow::domain::{Priority, Task, TaskStatus};
    use types::*;

    // ========================================================================
    // Helper Functions
    // ========================================================================

    /// Create a test model with sample data.
    fn create_test_model() -> Model {
        let mut model = Model::new();

        // Add some test tasks
        let task1 = Task::new("Test task 1")
            .with_priority(Priority::High)
            .with_tags(vec!["test".to_string(), "urgent".to_string()]);
        let task2 = Task::new("Test task 2").with_priority(Priority::Low);
        let mut task3 = Task::new("Completed task");
        task3.status = TaskStatus::Done;

        model.tasks.insert(task1.id, task1);
        model.tasks.insert(task2.id, task2);
        model.tasks.insert(task3.id, task3);
        model.refresh_visible_tasks();

        model
    }

    /// Parse a JSON request from a string.
    fn parse_request(json: &str) -> Result<PipeRequest, serde_json::Error> {
        serde_json::from_str(json)
    }

    // ========================================================================
    // Request Parsing Tests
    // ========================================================================

    #[test]
    fn test_parse_list_request() {
        let json = r#"{"operation":"list","entity":"task"}"#;
        let req = parse_request(json).unwrap();
        assert_eq!(req.operation, Operation::List);
        assert_eq!(req.entity, EntityType::Task);
        assert!(req.id.is_none());
        assert!(req.data.is_none());
    }

    #[test]
    fn test_parse_create_request() {
        let json = r#"{"operation":"create","entity":"task","data":{"title":"Buy milk"}}"#;
        let req = parse_request(json).unwrap();
        assert_eq!(req.operation, Operation::Create);
        assert_eq!(req.entity, EntityType::Task);
        assert!(req.data.is_some());

        let data = req.data.unwrap();
        assert_eq!(data.get("title").and_then(|v| v.as_str()), Some("Buy milk"));
    }

    #[test]
    fn test_parse_get_request() {
        let json = r#"{"operation":"get","entity":"task","id":"123"}"#;
        let req = parse_request(json).unwrap();
        assert_eq!(req.operation, Operation::Get);
        assert_eq!(req.entity, EntityType::Task);
        assert_eq!(req.id, Some("123".to_string()));
    }

    #[test]
    fn test_parse_update_request() {
        let json = r#"{"operation":"update","entity":"task","id":"123","data":{"status":"done"}}"#;
        let req = parse_request(json).unwrap();
        assert_eq!(req.operation, Operation::Update);
        assert_eq!(req.entity, EntityType::Task);
        assert_eq!(req.id, Some("123".to_string()));
        assert!(req.data.is_some());
    }

    #[test]
    fn test_parse_delete_request() {
        let json = r#"{"operation":"delete","entity":"task","id":"123"}"#;
        let req = parse_request(json).unwrap();
        assert_eq!(req.operation, Operation::Delete);
        assert_eq!(req.entity, EntityType::Task);
        assert_eq!(req.id, Some("123".to_string()));
    }

    #[test]
    fn test_parse_request_with_filters() {
        let json = r#"{
            "operation":"list",
            "entity":"task",
            "filters":{
                "status":["todo","in_progress"],
                "priority":["high","urgent"],
                "tags":["bug"],
                "project_id":"proj123",
                "limit":10,
                "offset":0
            }
        }"#;
        let req = parse_request(json).unwrap();
        assert_eq!(req.operation, Operation::List);

        let filters = req.filters.unwrap();
        assert_eq!(
            filters.status,
            Some(vec!["todo".to_string(), "in_progress".to_string()])
        );
        assert_eq!(
            filters.priority,
            Some(vec!["high".to_string(), "urgent".to_string()])
        );
        assert_eq!(filters.tags, Some(vec!["bug".to_string()]));
        assert_eq!(filters.project_id, Some("proj123".to_string()));
        assert_eq!(filters.limit, Some(10));
        assert_eq!(filters.offset, Some(0));
    }

    // ========================================================================
    // Malformed Request Tests
    // ========================================================================

    #[test]
    fn test_parse_empty_json() {
        let json = "";
        let result = parse_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_json() {
        let json = "not valid json{";
        let result = parse_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_operation() {
        let json = r#"{"entity":"task"}"#;
        let result = parse_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_entity() {
        let json = r#"{"operation":"list"}"#;
        let result = parse_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_operation() {
        let json = r#"{"operation":"invalid_op","entity":"task"}"#;
        let result = parse_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_entity() {
        let json = r#"{"operation":"list","entity":"invalid_entity"}"#;
        let result = parse_request(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_malformed_data() {
        // Data field is a string instead of object
        let json = r#"{"operation":"create","entity":"task","data":"not an object"}"#;
        let req = parse_request(json).unwrap();
        assert!(req.data.is_some());
        // The data will be parsed as JSON string value
        assert_eq!(req.data.unwrap(), json!("not an object"));
    }

    // ========================================================================
    // Entity Type Tests
    // ========================================================================

    #[test]
    fn test_all_entity_types() {
        let entities = [
            "task",
            "project",
            "time_entry",
            "work_log",
            "habit",
            "goal",
            "key_result",
            "tag",
            "saved_filter",
        ];

        for entity in entities {
            let json = format!(r#"{{"operation":"list","entity":"{}"}}"#, entity);
            let result = parse_request(&json);
            assert!(result.is_ok(), "Failed to parse entity: {}", entity);
        }
    }

    #[test]
    fn test_entity_type_case_sensitive() {
        // Entity types should be snake_case
        let json = r#"{"operation":"list","entity":"Task"}"#;
        let result = parse_request(json);
        assert!(result.is_err());
    }

    // ========================================================================
    // Operation Type Tests
    // ========================================================================

    #[test]
    fn test_all_operations() {
        let operations = [
            ("list", Operation::List),
            ("get", Operation::Get),
            ("create", Operation::Create),
            ("update", Operation::Update),
            ("delete", Operation::Delete),
            ("export", Operation::Export),
            ("import", Operation::Import),
        ];

        for (op_str, expected_op) in operations {
            let json = format!(r#"{{"operation":"{}","entity":"task"}}"#, op_str);
            let req = parse_request(&json).unwrap();
            assert_eq!(req.operation, expected_op);
        }
    }

    // ========================================================================
    // Response Serialization Tests
    // ========================================================================

    #[test]
    fn test_success_response_json() {
        let response = PipeResponse::success(json!({"id": "123", "title": "Test"}));
        let json = serde_json::to_string(&response).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["success"], true);
        assert_eq!(parsed["data"]["id"], "123");
        assert_eq!(parsed["data"]["title"], "Test");
        assert!(parsed.get("error").is_none());
    }

    #[test]
    fn test_error_response_json() {
        let response: PipeResponse<()> = PipeResponse::error("NOT_FOUND", "Task not found");
        let json = serde_json::to_string(&response).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["success"], false);
        assert_eq!(parsed["error"]["code"], "NOT_FOUND");
        assert_eq!(parsed["error"]["message"], "Task not found");
        assert!(parsed.get("data").is_none());
    }

    #[test]
    fn test_response_with_metadata() {
        let metadata = ResponseMetadata {
            total: Some(100),
            offset: Some(0),
            limit: Some(10),
        };
        let response = PipeResponse::success_with_metadata(json!([1, 2, 3]), metadata);
        let json = serde_json::to_string(&response).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["success"], true);
        assert_eq!(parsed["metadata"]["total"], 100);
        assert_eq!(parsed["metadata"]["offset"], 0);
        assert_eq!(parsed["metadata"]["limit"], 10);
    }

    // ========================================================================
    // Output Format Tests
    // ========================================================================

    #[test]
    fn test_output_format_parse_json() {
        assert_eq!(OutputFormat::parse("json"), Some(OutputFormat::Json));
        assert_eq!(OutputFormat::parse("JSON"), Some(OutputFormat::Json));
    }

    #[test]
    fn test_output_format_parse_yaml() {
        assert_eq!(OutputFormat::parse("yaml"), Some(OutputFormat::Yaml));
        assert_eq!(OutputFormat::parse("YAML"), Some(OutputFormat::Yaml));
        assert_eq!(OutputFormat::parse("yml"), Some(OutputFormat::Yaml));
    }

    #[test]
    fn test_output_format_parse_csv() {
        assert_eq!(OutputFormat::parse("csv"), Some(OutputFormat::Csv));
        assert_eq!(OutputFormat::parse("CSV"), Some(OutputFormat::Csv));
    }

    #[test]
    fn test_output_format_parse_invalid() {
        assert_eq!(OutputFormat::parse("invalid"), None);
        assert_eq!(OutputFormat::parse("xml"), None);
    }

    // ========================================================================
    // Filter Params Tests
    // ========================================================================

    #[test]
    fn test_filter_params_defaults() {
        let params: FilterParams = serde_json::from_str("{}").unwrap();
        assert!(params.project_id.is_none());
        assert!(params.tags.is_none());
        assert!(params.status.is_none());
        assert!(params.priority.is_none());
        assert!(params.search.is_none());
        assert!(params.limit.is_none());
        assert!(params.offset.is_none());
    }

    #[test]
    fn test_filter_params_all_fields() {
        let json = r#"{
            "project_id": "proj1",
            "tags": ["bug", "urgent"],
            "tags_mode": "any",
            "status": ["todo"],
            "priority": ["high"],
            "search": "fix",
            "due_before": "2025-12-31",
            "due_after": "2025-01-01",
            "include_completed": true,
            "limit": 50,
            "offset": 10,
            "sort_by": "due_date",
            "sort_order": "asc"
        }"#;

        let params: FilterParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.project_id, Some("proj1".to_string()));
        assert_eq!(
            params.tags,
            Some(vec!["bug".to_string(), "urgent".to_string()])
        );
        assert_eq!(params.tags_mode, Some("any".to_string()));
        assert_eq!(params.status, Some(vec!["todo".to_string()]));
        assert_eq!(params.priority, Some(vec!["high".to_string()]));
        assert_eq!(params.search, Some("fix".to_string()));
        assert_eq!(params.due_before, Some("2025-12-31".to_string()));
        assert_eq!(params.due_after, Some("2025-01-01".to_string()));
        assert_eq!(params.include_completed, Some(true));
        assert_eq!(params.limit, Some(50));
        assert_eq!(params.offset, Some(10));
        assert_eq!(params.sort_by, Some("due_date".to_string()));
        assert_eq!(params.sort_order, Some("asc".to_string()));
    }

    // ========================================================================
    // Input Type Tests
    // ========================================================================

    #[test]
    fn test_task_input_minimal() {
        let json = r#"{"title":"Test task"}"#;
        let input: TaskInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.title, Some("Test task".to_string()));
        assert!(input.description.is_none());
        assert!(input.status.is_none());
    }

    #[test]
    fn test_task_input_full() {
        let json = r#"{
            "title": "Full task",
            "description": "Detailed description",
            "status": "in_progress",
            "priority": "high",
            "project_id": "proj1",
            "tags": ["bug", "urgent"],
            "due_date": "2025-12-31",
            "scheduled_date": "2025-01-15",
            "estimated_minutes": 120,
            "dependencies": ["task1", "task2"]
        }"#;

        let input: TaskInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.title, Some("Full task".to_string()));
        assert_eq!(input.description, Some("Detailed description".to_string()));
        assert_eq!(input.status, Some("in_progress".to_string()));
        assert_eq!(input.priority, Some("high".to_string()));
        assert_eq!(input.project_id, Some("proj1".to_string()));
        assert_eq!(
            input.tags,
            Some(vec!["bug".to_string(), "urgent".to_string()])
        );
        assert_eq!(input.due_date, Some("2025-12-31".to_string()));
        assert_eq!(input.scheduled_date, Some("2025-01-15".to_string()));
        assert_eq!(input.estimated_minutes, Some(120));
        assert_eq!(
            input.dependencies,
            Some(vec!["task1".to_string(), "task2".to_string()])
        );
    }

    #[test]
    fn test_project_input() {
        let json = r#"{
            "name": "Test Project",
            "description": "Project description",
            "status": "active",
            "parent_id": "parent1",
            "color": "FF5733"
        }"#;

        let input: ProjectInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, Some("Test Project".to_string()));
        assert_eq!(input.description, Some("Project description".to_string()));
        assert_eq!(input.status, Some("active".to_string()));
        assert_eq!(input.parent_id, Some("parent1".to_string()));
        assert_eq!(input.color, Some("FF5733".to_string()));
    }

    #[test]
    fn test_time_entry_input() {
        let json = r#"{
            "task_id": "task1",
            "started_at": "2025-01-15T10:00:00Z",
            "ended_at": "2025-01-15T11:30:00Z",
            "description": "Worked on bug fix"
        }"#;

        let input: TimeEntryInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.task_id, Some("task1".to_string()));
        assert_eq!(input.started_at, Some("2025-01-15T10:00:00Z".to_string()));
        assert_eq!(input.ended_at, Some("2025-01-15T11:30:00Z".to_string()));
        assert_eq!(input.description, Some("Worked on bug fix".to_string()));
    }

    #[test]
    fn test_time_entry_input_with_duration() {
        let json = r#"{
            "task_id": "task1",
            "started_at": "2025-01-15T10:00:00Z",
            "duration_minutes": 90
        }"#;

        let input: TimeEntryInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.duration_minutes, Some(90));
        assert!(input.ended_at.is_none());
    }

    #[test]
    fn test_habit_input() {
        let json = r#"{
            "name": "Exercise",
            "description": "Daily workout",
            "frequency": "daily",
            "target_count": 1
        }"#;

        let input: HabitInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, Some("Exercise".to_string()));
        assert_eq!(input.description, Some("Daily workout".to_string()));
        assert_eq!(input.frequency, Some("daily".to_string()));
        assert_eq!(input.target_count, Some(1));
    }

    #[test]
    fn test_goal_input() {
        let json = r#"{
            "name": "Q1 Goals",
            "description": "Complete all Q1 objectives",
            "target_date": "2025-03-31"
        }"#;

        let input: GoalInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, Some("Q1 Goals".to_string()));
        assert_eq!(
            input.description,
            Some("Complete all Q1 objectives".to_string())
        );
        assert_eq!(input.target_date, Some("2025-03-31".to_string()));
    }

    #[test]
    fn test_key_result_input() {
        let json = r#"{
            "goal_id": "goal1",
            "name": "Increase test coverage",
            "target_value": 80.0,
            "current_value": 65.0,
            "unit": "%"
        }"#;

        let input: KeyResultInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.goal_id, Some("goal1".to_string()));
        assert_eq!(input.name, Some("Increase test coverage".to_string()));
        assert_eq!(input.target_value, Some(80.0));
        assert_eq!(input.current_value, Some(65.0));
        assert_eq!(input.unit, Some("%".to_string()));
    }

    // ========================================================================
    // Error Type Tests
    // ========================================================================

    #[test]
    fn test_pipe_error_new() {
        let error = PipeError::new("TEST_ERROR", "This is a test error");
        assert_eq!(error.code, "TEST_ERROR");
        assert_eq!(error.message, "This is a test error");
        assert!(error.details.is_none());
    }

    #[test]
    fn test_pipe_error_serialization() {
        let error = PipeError::serialization("Invalid UTF-8");
        assert_eq!(error.code, "SERIALIZATION_ERROR");
        assert!(error.message.contains("Failed to serialize"));
        assert!(error.message.contains("Invalid UTF-8"));
    }

    #[test]
    fn test_pipe_error_display() {
        let error = PipeError::new("NOT_FOUND", "Task not found");
        let display = format!("{}", error);
        assert_eq!(display, "NOT_FOUND: Task not found");
    }

    // ========================================================================
    // Integration Tests - Process Requests
    // ========================================================================

    #[test]
    fn test_process_list_request() {
        let mut model = create_test_model();
        let request = PipeRequest {
            operation: Operation::List,
            entity: EntityType::Task,
            id: None,
            data: None,
            filters: None,
        };

        let response = super::super::process_request(&mut model, request);
        assert!(response.success);
        assert!(response.data.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_process_create_request() {
        let mut model = create_test_model();
        let task_count_before = model.tasks.len();

        let request = PipeRequest {
            operation: Operation::Create,
            entity: EntityType::Task,
            id: None,
            data: Some(json!({"title": "New task via pipe"})),
            filters: None,
        };

        let response = super::super::process_request(&mut model, request);
        assert!(response.success);
        assert!(response.error.is_none());

        let task_count_after = model.tasks.len();
        assert_eq!(task_count_after, task_count_before + 1);
    }

    #[test]
    fn test_process_get_request_existing() {
        let mut model = create_test_model();
        let task_id = model.tasks.values().next().unwrap().id;

        let request = PipeRequest {
            operation: Operation::Get,
            entity: EntityType::Task,
            id: Some(task_id.to_string()),
            data: None,
            filters: None,
        };

        let response = super::super::process_request(&mut model, request);
        assert!(response.success);
        assert!(response.data.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_process_get_request_nonexistent() {
        let mut model = create_test_model();

        let request = PipeRequest {
            operation: Operation::Get,
            entity: EntityType::Task,
            id: Some("nonexistent-id".to_string()),
            data: None,
            filters: None,
        };

        let response = super::super::process_request(&mut model, request);
        assert!(!response.success);
        assert!(response.error.is_some());
    }

    #[test]
    fn test_process_delete_request() {
        let mut model = create_test_model();
        let task_count_before = model.tasks.len();
        let task_id = model.tasks.values().next().unwrap().id;

        let request = PipeRequest {
            operation: Operation::Delete,
            entity: EntityType::Task,
            id: Some(task_id.to_string()),
            data: None,
            filters: None,
        };

        let response = super::super::process_request(&mut model, request);
        assert!(response.success);

        let task_count_after = model.tasks.len();
        assert_eq!(task_count_after, task_count_before - 1);
    }

    // ========================================================================
    // Edge Cases and Stress Tests
    // ========================================================================

    #[test]
    fn test_unicode_in_task_title() {
        let json = r#"{"operation":"create","entity":"task","data":{"title":"日本語 タスク 🎉"}}"#;
        let req = parse_request(json).unwrap();
        let data = req.data.unwrap();
        assert_eq!(
            data.get("title").and_then(|v| v.as_str()),
            Some("日本語 タスク 🎉")
        );
    }

    #[test]
    fn test_very_long_title() {
        let long_title = "A".repeat(10000);
        let json = format!(
            r#"{{"operation":"create","entity":"task","data":{{"title":"{}"}}}}"#,
            long_title
        );
        let req = parse_request(&json).unwrap();
        let data = req.data.unwrap();
        assert_eq!(
            data.get("title").and_then(|v| v.as_str()).unwrap().len(),
            10000
        );
    }

    #[test]
    fn test_special_characters_in_data() {
        let json = r#"{"operation":"create","entity":"task","data":{"title":"Task with \"quotes\" and \n newlines"}}"#;
        let req = parse_request(json).unwrap();
        let data = req.data.unwrap();
        assert!(data
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap()
            .contains("quotes"));
    }

    #[test]
    fn test_empty_filters() {
        let json = r#"{"operation":"list","entity":"task","filters":{}}"#;
        let req = parse_request(json).unwrap();
        assert!(req.filters.is_some());
        let filters = req.filters.unwrap();
        assert!(filters.status.is_none());
        assert!(filters.priority.is_none());
    }

    #[test]
    fn test_null_data_field() {
        let json = r#"{"operation":"create","entity":"task","data":null}"#;
        let req = parse_request(json).unwrap();
        assert!(req.data.is_none());
    }

    #[test]
    fn test_extra_fields_ignored() {
        let json =
            r#"{"operation":"list","entity":"task","unknown_field":"value","another_field":123}"#;
        let req = parse_request(json).unwrap();
        assert_eq!(req.operation, Operation::List);
        // Extra fields should be ignored during deserialization
    }

    #[test]
    fn test_number_as_string_id() {
        let json = r#"{"operation":"get","entity":"task","id":"12345"}"#;
        let req = parse_request(json).unwrap();
        assert_eq!(req.id, Some("12345".to_string()));
    }

    #[test]
    fn test_empty_string_id() {
        let json = r#"{"operation":"get","entity":"task","id":""}"#;
        let req = parse_request(json).unwrap();
        assert_eq!(req.id, Some("".to_string()));
    }

    #[test]
    fn test_nested_json_in_data() {
        let json = r#"{
            "operation":"create",
            "entity":"task",
            "data":{
                "title":"Task",
                "metadata":{"custom":{"nested":"value"}}
            }
        }"#;
        let req = parse_request(json).unwrap();
        let data = req.data.unwrap();
        assert!(data.get("metadata").is_some());
    }

    // ========================================================================
    // Goal Handler Tests
    // ========================================================================

    fn make_request(operation: Operation, entity: EntityType) -> PipeRequest {
        PipeRequest {
            operation,
            entity,
            id: None,
            data: None,
            filters: None,
        }
    }

    fn make_request_with_data(
        operation: Operation,
        entity: EntityType,
        data: serde_json::Value,
    ) -> PipeRequest {
        PipeRequest {
            operation,
            entity,
            id: None,
            data: Some(data),
            filters: None,
        }
    }

    fn make_request_with_id(operation: Operation, entity: EntityType, id: &str) -> PipeRequest {
        PipeRequest {
            operation,
            entity,
            id: Some(id.to_string()),
            data: None,
            filters: None,
        }
    }

    fn make_request_with_id_and_data(
        operation: Operation,
        entity: EntityType,
        id: &str,
        data: serde_json::Value,
    ) -> PipeRequest {
        PipeRequest {
            operation,
            entity,
            id: Some(id.to_string()),
            data: Some(data),
            filters: None,
        }
    }

    #[test]
    fn test_goal_list_empty() {
        let mut model = Model::new();
        let req = make_request(Operation::List, EntityType::Goal);
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        let data = resp.data.unwrap();
        assert_eq!(data["total"], 0);
    }

    #[test]
    fn test_goal_create_and_list() {
        let mut model = Model::new();
        let req = make_request_with_data(
            Operation::Create,
            EntityType::Goal,
            json!({"name": "Q1 Goals", "description": "First quarter objectives"}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success, "create failed: {:?}", resp.error);
        let data = resp.data.unwrap();
        assert_eq!(data["name"], "Q1 Goals");
        let goal_id = data["id"].as_str().unwrap().to_string();

        // List should return 1 goal
        let req = make_request(Operation::List, EntityType::Goal);
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        assert_eq!(resp.data.unwrap()["total"], 1);

        // Get the goal by id
        let req = make_request_with_id(Operation::Get, EntityType::Goal, &goal_id);
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        assert_eq!(resp.data.unwrap()["name"], "Q1 Goals");
    }

    #[test]
    fn test_goal_create_missing_name() {
        let mut model = Model::new();
        let req = make_request_with_data(
            Operation::Create,
            EntityType::Goal,
            json!({"description": "No name"}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_DATA");
    }

    #[test]
    fn test_goal_create_no_data() {
        let mut model = Model::new();
        let req = make_request(Operation::Create, EntityType::Goal);
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_DATA");
    }

    #[test]
    fn test_goal_get_not_found() {
        let mut model = Model::new();
        let req = make_request_with_id(
            Operation::Get,
            EntityType::Goal,
            "00000000-0000-0000-0000-000000000000",
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "NOT_FOUND");
    }

    #[test]
    fn test_goal_get_invalid_uuid() {
        let mut model = Model::new();
        let req = make_request_with_id(Operation::Get, EntityType::Goal, "not-a-uuid");
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "INVALID_ID");
    }

    #[test]
    fn test_goal_get_missing_id() {
        let mut model = Model::new();
        let req = make_request(Operation::Get, EntityType::Goal);
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_ID");
    }

    #[test]
    fn test_goal_update() {
        let mut model = Model::new();
        // Create goal
        let req = make_request_with_data(
            Operation::Create,
            EntityType::Goal,
            json!({"name": "Original"}),
        );
        let resp = super::super::process_request(&mut model, req);
        let goal_id = resp.data.unwrap()["id"].as_str().unwrap().to_string();

        // Update it
        let req = make_request_with_id_and_data(
            Operation::Update,
            EntityType::Goal,
            &goal_id,
            json!({"name": "Updated", "description": "new desc", "status": "on_hold"}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success, "{:?}", resp.error);
        assert_eq!(resp.data.unwrap()["name"], "Updated");
    }

    #[test]
    fn test_goal_update_not_found() {
        let mut model = Model::new();
        let req = make_request_with_id_and_data(
            Operation::Update,
            EntityType::Goal,
            "00000000-0000-0000-0000-000000000000",
            json!({"name": "X"}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "NOT_FOUND");
    }

    #[test]
    fn test_goal_update_missing_id() {
        let mut model = Model::new();
        let req = make_request_with_data(Operation::Update, EntityType::Goal, json!({"name": "X"}));
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_ID");
    }

    #[test]
    fn test_goal_delete() {
        let mut model = Model::new();
        let req = make_request_with_data(
            Operation::Create,
            EntityType::Goal,
            json!({"name": "To Delete"}),
        );
        let resp = super::super::process_request(&mut model, req);
        let goal_id = resp.data.unwrap()["id"].as_str().unwrap().to_string();

        let req = make_request_with_id(Operation::Delete, EntityType::Goal, &goal_id);
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        assert_eq!(model.goals.len(), 0);
    }

    #[test]
    fn test_goal_delete_not_found() {
        let mut model = Model::new();
        let req = make_request_with_id(
            Operation::Delete,
            EntityType::Goal,
            "00000000-0000-0000-0000-000000000000",
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "NOT_FOUND");
    }

    #[test]
    fn test_goal_delete_missing_id() {
        let mut model = Model::new();
        let req = make_request(Operation::Delete, EntityType::Goal);
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_ID");
    }

    #[test]
    fn test_saved_filter_not_implemented() {
        let mut model = Model::new();
        let req = make_request(Operation::List, EntityType::SavedFilter);
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "NOT_IMPLEMENTED");
    }

    #[test]
    fn test_goal_list_with_filters() {
        let mut model = Model::new();
        // Create two goals
        super::super::process_request(
            &mut model,
            make_request_with_data(
                Operation::Create,
                EntityType::Goal,
                json!({"name": "Active Goal"}),
            ),
        );
        super::super::process_request(
            &mut model,
            make_request_with_data(
                Operation::Create,
                EntityType::Goal,
                json!({"name": "Another Goal"}),
            ),
        );

        // List with search filter
        let req = PipeRequest {
            operation: Operation::List,
            entity: EntityType::Goal,
            id: None,
            data: None,
            filters: Some(FilterParams {
                search: Some("Active".to_string()),
                limit: Some(10),
                ..Default::default()
            }),
        };
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        assert_eq!(resp.data.unwrap()["total"], 1);
    }

    // ========================================================================
    // KeyResult Handler Tests
    // ========================================================================

    fn create_goal_in_model(model: &mut Model) -> String {
        let req = make_request_with_data(
            Operation::Create,
            EntityType::Goal,
            json!({"name": "Test Goal"}),
        );
        let resp = super::super::process_request(model, req);
        resp.data.unwrap()["id"].as_str().unwrap().to_string()
    }

    #[test]
    fn test_key_result_list_empty() {
        let mut model = Model::new();
        let req = make_request(Operation::List, EntityType::KeyResult);
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        assert_eq!(resp.data.unwrap()["total"], 0);
    }

    #[test]
    fn test_key_result_create_and_get() {
        let mut model = Model::new();
        let goal_id = create_goal_in_model(&mut model);

        let req = make_request_with_data(
            Operation::Create,
            EntityType::KeyResult,
            json!({"goal_id": goal_id, "name": "Increase coverage", "target_value": 80.0, "unit": "%"}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success, "{:?}", resp.error);
        let kr_id = resp.data.unwrap()["id"].as_str().unwrap().to_string();

        let req = make_request_with_id(Operation::Get, EntityType::KeyResult, &kr_id);
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        assert_eq!(resp.data.unwrap()["name"], "Increase coverage");
    }

    #[test]
    fn test_key_result_create_missing_goal_id() {
        let mut model = Model::new();
        let req = make_request_with_data(
            Operation::Create,
            EntityType::KeyResult,
            json!({"name": "No goal"}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_DATA");
    }

    #[test]
    fn test_key_result_create_missing_name() {
        let mut model = Model::new();
        let goal_id = create_goal_in_model(&mut model);
        let req = make_request_with_data(
            Operation::Create,
            EntityType::KeyResult,
            json!({"goal_id": goal_id}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_DATA");
    }

    #[test]
    fn test_key_result_create_goal_not_found() {
        let mut model = Model::new();
        let req = make_request_with_data(
            Operation::Create,
            EntityType::KeyResult,
            json!({"goal_id": "00000000-0000-0000-0000-000000000000", "name": "KR"}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "NOT_FOUND");
    }

    #[test]
    fn test_key_result_update() {
        let mut model = Model::new();
        let goal_id = create_goal_in_model(&mut model);

        let req = make_request_with_data(
            Operation::Create,
            EntityType::KeyResult,
            json!({"goal_id": goal_id, "name": "KR", "target_value": 100.0}),
        );
        let resp = super::super::process_request(&mut model, req);
        let kr_id = resp.data.unwrap()["id"].as_str().unwrap().to_string();

        let req = make_request_with_id_and_data(
            Operation::Update,
            EntityType::KeyResult,
            &kr_id,
            json!({"name": "Updated KR", "current_value": 50.0, "target_value": 90.0}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success, "{:?}", resp.error);
        assert_eq!(resp.data.as_ref().unwrap()["name"], "Updated KR");
        assert_eq!(resp.data.as_ref().unwrap()["current_value"], 50.0);
    }

    #[test]
    fn test_key_result_delete() {
        let mut model = Model::new();
        let goal_id = create_goal_in_model(&mut model);

        let req = make_request_with_data(
            Operation::Create,
            EntityType::KeyResult,
            json!({"goal_id": goal_id, "name": "KR to delete"}),
        );
        let resp = super::super::process_request(&mut model, req);
        let kr_id = resp.data.unwrap()["id"].as_str().unwrap().to_string();

        let req = make_request_with_id(Operation::Delete, EntityType::KeyResult, &kr_id);
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        assert_eq!(model.key_results.len(), 0);
    }

    #[test]
    fn test_key_result_delete_not_found() {
        let mut model = Model::new();
        let req = make_request_with_id(
            Operation::Delete,
            EntityType::KeyResult,
            "00000000-0000-0000-0000-000000000000",
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "NOT_FOUND");
    }

    #[test]
    fn test_key_result_get_not_found() {
        let mut model = Model::new();
        let req = make_request_with_id(
            Operation::Get,
            EntityType::KeyResult,
            "00000000-0000-0000-0000-000000000000",
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "NOT_FOUND");
    }

    #[test]
    fn test_key_result_update_missing_data() {
        let mut model = Model::new();
        let req = make_request_with_id(
            Operation::Update,
            EntityType::KeyResult,
            "00000000-0000-0000-0000-000000000000",
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_DATA");
    }

    #[test]
    fn test_goal_delete_cascades_key_results() {
        let mut model = Model::new();
        let goal_id = create_goal_in_model(&mut model);

        // Create key result
        super::super::process_request(
            &mut model,
            make_request_with_data(
                Operation::Create,
                EntityType::KeyResult,
                json!({"goal_id": goal_id, "name": "KR"}),
            ),
        );
        assert_eq!(model.key_results.len(), 1);

        // Delete goal
        let req = make_request_with_id(Operation::Delete, EntityType::Goal, &goal_id);
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        assert_eq!(resp.data.unwrap()["deleted_key_results"], 1);
        assert_eq!(model.key_results.len(), 0);
    }

    // ========================================================================
    // Habit Handler Tests
    // ========================================================================

    #[test]
    fn test_habit_list_empty() {
        let mut model = Model::new();
        let req = make_request(Operation::List, EntityType::Habit);
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        assert_eq!(resp.data.unwrap()["total"], 0);
    }

    #[test]
    fn test_habit_create_and_list() {
        let mut model = Model::new();
        let req = make_request_with_data(
            Operation::Create,
            EntityType::Habit,
            json!({"name": "Exercise", "description": "Daily workout", "frequency": "daily"}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success, "{:?}", resp.error);
        assert_eq!(resp.data.unwrap()["name"], "Exercise");

        let req = make_request(Operation::List, EntityType::Habit);
        let resp = super::super::process_request(&mut model, req);
        assert_eq!(resp.data.unwrap()["total"], 1);
    }

    #[test]
    fn test_habit_create_missing_name() {
        let mut model = Model::new();
        let req = make_request_with_data(
            Operation::Create,
            EntityType::Habit,
            json!({"frequency": "daily"}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_DATA");
    }

    #[test]
    fn test_habit_create_no_data() {
        let mut model = Model::new();
        let req = make_request(Operation::Create, EntityType::Habit);
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_DATA");
    }

    #[test]
    fn test_habit_create_with_weekly_frequency() {
        let mut model = Model::new();
        let req = make_request_with_data(
            Operation::Create,
            EntityType::Habit,
            json!({"name": "Weekly habit", "frequency": "weekly"}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
    }

    #[test]
    fn test_habit_create_with_every_n_days_frequency() {
        let mut model = Model::new();
        let req = make_request_with_data(
            Operation::Create,
            EntityType::Habit,
            json!({"name": "Every 3 days", "frequency": "every_3_days"}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
    }

    #[test]
    fn test_habit_get() {
        let mut model = Model::new();
        let req = make_request_with_data(
            Operation::Create,
            EntityType::Habit,
            json!({"name": "Meditate"}),
        );
        let resp = super::super::process_request(&mut model, req);
        let habit_id = resp.data.unwrap()["id"].as_str().unwrap().to_string();

        let req = make_request_with_id(Operation::Get, EntityType::Habit, &habit_id);
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        assert_eq!(resp.data.unwrap()["name"], "Meditate");
    }

    #[test]
    fn test_habit_get_not_found() {
        let mut model = Model::new();
        let req = make_request_with_id(
            Operation::Get,
            EntityType::Habit,
            "00000000-0000-0000-0000-000000000000",
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "NOT_FOUND");
    }

    #[test]
    fn test_habit_get_missing_id() {
        let mut model = Model::new();
        let req = make_request(Operation::Get, EntityType::Habit);
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_ID");
    }

    #[test]
    fn test_habit_update() {
        let mut model = Model::new();
        let req = make_request_with_data(
            Operation::Create,
            EntityType::Habit,
            json!({"name": "Walk"}),
        );
        let resp = super::super::process_request(&mut model, req);
        let habit_id = resp.data.unwrap()["id"].as_str().unwrap().to_string();

        let req = make_request_with_id_and_data(
            Operation::Update,
            EntityType::Habit,
            &habit_id,
            json!({"name": "Run", "description": "5km run", "frequency": "daily", "archived": false}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success, "{:?}", resp.error);
        assert_eq!(resp.data.unwrap()["name"], "Run");
    }

    #[test]
    fn test_habit_update_not_found() {
        let mut model = Model::new();
        let req = make_request_with_id_and_data(
            Operation::Update,
            EntityType::Habit,
            "00000000-0000-0000-0000-000000000000",
            json!({"name": "X"}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "NOT_FOUND");
    }

    #[test]
    fn test_habit_delete() {
        let mut model = Model::new();
        let req = make_request_with_data(
            Operation::Create,
            EntityType::Habit,
            json!({"name": "To Delete"}),
        );
        let resp = super::super::process_request(&mut model, req);
        let habit_id = resp.data.unwrap()["id"].as_str().unwrap().to_string();

        let req = make_request_with_id(Operation::Delete, EntityType::Habit, &habit_id);
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        assert_eq!(model.habits.len(), 0);
    }

    #[test]
    fn test_habit_delete_not_found() {
        let mut model = Model::new();
        let req = make_request_with_id(
            Operation::Delete,
            EntityType::Habit,
            "00000000-0000-0000-0000-000000000000",
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "NOT_FOUND");
    }

    #[test]
    fn test_habit_update_missing_data() {
        let mut model = Model::new();
        let req = make_request_with_id(
            Operation::Update,
            EntityType::Habit,
            "00000000-0000-0000-0000-000000000000",
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_DATA");
    }

    #[test]
    fn test_habit_list_with_search_filter() {
        let mut model = Model::new();
        super::super::process_request(
            &mut model,
            make_request_with_data(
                Operation::Create,
                EntityType::Habit,
                json!({"name": "Exercise"}),
            ),
        );
        super::super::process_request(
            &mut model,
            make_request_with_data(
                Operation::Create,
                EntityType::Habit,
                json!({"name": "Meditate"}),
            ),
        );

        let req = PipeRequest {
            operation: Operation::List,
            entity: EntityType::Habit,
            id: None,
            data: None,
            filters: Some(FilterParams {
                search: Some("Exer".to_string()),
                ..Default::default()
            }),
        };
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        assert_eq!(resp.data.unwrap()["total"], 1);
    }

    // ========================================================================
    // Project Handler Tests
    // ========================================================================

    #[test]
    fn test_project_list_empty() {
        let mut model = Model::new();
        let req = make_request(Operation::List, EntityType::Project);
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        assert_eq!(resp.data.unwrap()["total"], 0);
    }

    #[test]
    fn test_project_create_and_list() {
        let mut model = Model::new();
        let req = make_request_with_data(
            Operation::Create,
            EntityType::Project,
            json!({"name": "TaskFlow", "description": "Main project", "status": "active"}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success, "{:?}", resp.error);
        assert_eq!(resp.data.unwrap()["name"], "TaskFlow");

        let req = make_request(Operation::List, EntityType::Project);
        let resp = super::super::process_request(&mut model, req);
        assert_eq!(resp.data.unwrap()["total"], 1);
    }

    #[test]
    fn test_project_create_with_parent() {
        let mut model = Model::new();
        let req = make_request_with_data(
            Operation::Create,
            EntityType::Project,
            json!({"name": "Parent Project"}),
        );
        let resp = super::super::process_request(&mut model, req);
        let parent_id = resp.data.unwrap()["id"].as_str().unwrap().to_string();

        let req = make_request_with_data(
            Operation::Create,
            EntityType::Project,
            json!({"name": "Child Project", "parent_id": parent_id}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
    }

    #[test]
    fn test_project_create_missing_name() {
        let mut model = Model::new();
        let req = make_request_with_data(
            Operation::Create,
            EntityType::Project,
            json!({"description": "No name"}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_DATA");
    }

    #[test]
    fn test_project_get() {
        let mut model = Model::new();
        let req = make_request_with_data(
            Operation::Create,
            EntityType::Project,
            json!({"name": "My Project"}),
        );
        let resp = super::super::process_request(&mut model, req);
        let project_id = resp.data.unwrap()["id"].as_str().unwrap().to_string();

        let req = make_request_with_id(Operation::Get, EntityType::Project, &project_id);
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        assert_eq!(resp.data.unwrap()["name"], "My Project");
    }

    #[test]
    fn test_project_get_not_found() {
        let mut model = Model::new();
        let req = make_request_with_id(
            Operation::Get,
            EntityType::Project,
            "00000000-0000-0000-0000-000000000000",
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "NOT_FOUND");
    }

    #[test]
    fn test_project_get_missing_id() {
        let mut model = Model::new();
        let req = make_request(Operation::Get, EntityType::Project);
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_ID");
    }

    #[test]
    fn test_project_update() {
        let mut model = Model::new();
        let req = make_request_with_data(
            Operation::Create,
            EntityType::Project,
            json!({"name": "Original"}),
        );
        let resp = super::super::process_request(&mut model, req);
        let project_id = resp.data.unwrap()["id"].as_str().unwrap().to_string();

        let req = make_request_with_id_and_data(
            Operation::Update,
            EntityType::Project,
            &project_id,
            json!({"name": "Updated", "description": "new desc", "status": "on_hold", "color": null}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success, "{:?}", resp.error);
        assert_eq!(resp.data.unwrap()["name"], "Updated");
    }

    #[test]
    fn test_project_update_not_found() {
        let mut model = Model::new();
        let req = make_request_with_id_and_data(
            Operation::Update,
            EntityType::Project,
            "00000000-0000-0000-0000-000000000000",
            json!({"name": "X"}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "NOT_FOUND");
    }

    #[test]
    fn test_project_update_missing_id() {
        let mut model = Model::new();
        let req =
            make_request_with_data(Operation::Update, EntityType::Project, json!({"name": "X"}));
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_ID");
    }

    #[test]
    fn test_project_delete() {
        let mut model = Model::new();
        let req = make_request_with_data(
            Operation::Create,
            EntityType::Project,
            json!({"name": "To Delete"}),
        );
        let resp = super::super::process_request(&mut model, req);
        let project_id = resp.data.unwrap()["id"].as_str().unwrap().to_string();

        let req = make_request_with_id(Operation::Delete, EntityType::Project, &project_id);
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        assert_eq!(model.projects.len(), 0);
    }

    #[test]
    fn test_project_delete_not_found() {
        let mut model = Model::new();
        let req = make_request_with_id(
            Operation::Delete,
            EntityType::Project,
            "00000000-0000-0000-0000-000000000000",
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "NOT_FOUND");
    }

    #[test]
    fn test_project_update_missing_data() {
        let mut model = Model::new();
        let req = make_request_with_id(
            Operation::Update,
            EntityType::Project,
            "00000000-0000-0000-0000-000000000000",
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_DATA");
    }

    #[test]
    fn test_project_list_with_status_filter() {
        let mut model = Model::new();
        super::super::process_request(
            &mut model,
            make_request_with_data(
                Operation::Create,
                EntityType::Project,
                json!({"name": "Active", "status": "active"}),
            ),
        );
        super::super::process_request(
            &mut model,
            make_request_with_data(
                Operation::Create,
                EntityType::Project,
                json!({"name": "Archived", "status": "archived"}),
            ),
        );

        let req = PipeRequest {
            operation: Operation::List,
            entity: EntityType::Project,
            id: None,
            data: None,
            filters: Some(FilterParams {
                status: Some(vec!["active".to_string()]),
                ..Default::default()
            }),
        };
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        assert_eq!(resp.data.unwrap()["total"], 1);
    }

    // ========================================================================
    // Tag Handler Tests
    // ========================================================================

    #[test]
    fn test_tag_list_empty() {
        let mut model = Model::new();
        let req = make_request(Operation::List, EntityType::Tag);
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        assert_eq!(resp.data.unwrap()["total"], 0);
    }

    #[test]
    fn test_tag_list_from_tasks() {
        let mut model = create_test_model(); // has tasks with "test" and "urgent" tags
        let req = make_request(Operation::List, EntityType::Tag);
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        let data = resp.data.unwrap();
        assert!(data["total"].as_u64().unwrap() >= 2);
    }

    #[test]
    fn test_tag_get_existing() {
        let mut model = create_test_model();
        let req = make_request_with_id(Operation::Get, EntityType::Tag, "test");
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        let data = resp.data.unwrap();
        assert_eq!(data["name"], "test");
        assert!(data["task_count"].as_u64().unwrap() >= 1);
    }

    #[test]
    fn test_tag_get_not_found() {
        let mut model = Model::new();
        let req = make_request_with_id(Operation::Get, EntityType::Tag, "nonexistent");
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "NOT_FOUND");
    }

    #[test]
    fn test_tag_get_missing_id() {
        let mut model = Model::new();
        let req = make_request(Operation::Get, EntityType::Tag);
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_ID");
    }

    #[test]
    fn test_tag_create_invalid() {
        let mut model = Model::new();
        let req =
            make_request_with_data(Operation::Create, EntityType::Tag, json!({"name": "test"}));
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "INVALID_OPERATION");
    }

    #[test]
    fn test_tag_update_invalid() {
        let mut model = Model::new();
        let req = make_request_with_id_and_data(
            Operation::Update,
            EntityType::Tag,
            "test",
            json!({"name": "x"}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "INVALID_OPERATION");
    }

    #[test]
    fn test_tag_delete_invalid() {
        let mut model = Model::new();
        let req = make_request_with_id(Operation::Delete, EntityType::Tag, "test");
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "INVALID_OPERATION");
    }

    #[test]
    fn test_tag_import_invalid() {
        let mut model = Model::new();
        // Import at entity level is intercepted by dispatch's import handler
        // Test that Create/Update/Delete still return INVALID_OPERATION for tags
        let req = PipeRequest {
            operation: Operation::Create,
            entity: EntityType::Tag,
            id: None,
            data: Some(json!({"name": "x"})),
            filters: None,
        };
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "INVALID_OPERATION");
    }

    #[test]
    fn test_tag_list_with_search_filter() {
        let mut model = create_test_model();
        let req = PipeRequest {
            operation: Operation::List,
            entity: EntityType::Tag,
            id: None,
            data: None,
            filters: Some(FilterParams {
                search: Some("test".to_string()),
                sort_by: Some("count".to_string()),
                sort_order: Some("desc".to_string()),
                ..Default::default()
            }),
        };
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        let data = resp.data.unwrap();
        assert!(data["total"].as_u64().unwrap() >= 1);
    }

    #[test]
    fn test_tag_list_sort_by_name_asc() {
        let mut model = create_test_model();
        let req = PipeRequest {
            operation: Operation::List,
            entity: EntityType::Tag,
            id: None,
            data: None,
            filters: Some(FilterParams {
                sort_by: Some("name".to_string()),
                sort_order: Some("asc".to_string()),
                ..Default::default()
            }),
        };
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
    }

    // ========================================================================
    // TimeEntry Handler Tests
    // ========================================================================

    fn create_task_in_model(model: &mut Model) -> String {
        use taskflow::domain::Task;
        let task = Task::new("Test task");
        let id = task.id.0.to_string();
        model.tasks.insert(task.id, task);
        id
    }

    #[test]
    fn test_time_entry_list_empty() {
        let mut model = Model::new();
        let req = make_request(Operation::List, EntityType::TimeEntry);
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        assert_eq!(resp.data.unwrap()["total"], 0);
    }

    #[test]
    fn test_time_entry_create_and_list() {
        let mut model = Model::new();
        let task_id = create_task_in_model(&mut model);

        let req = make_request_with_data(
            Operation::Create,
            EntityType::TimeEntry,
            json!({
                "task_id": task_id,
                "started_at": "2025-01-15T10:00:00Z",
                "ended_at": "2025-01-15T11:30:00Z",
                "description": "Bug fix"
            }),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success, "{:?}", resp.error);

        let req = make_request(Operation::List, EntityType::TimeEntry);
        let resp = super::super::process_request(&mut model, req);
        assert_eq!(resp.data.unwrap()["total"], 1);
    }

    #[test]
    fn test_time_entry_create_with_duration() {
        let mut model = Model::new();
        let task_id = create_task_in_model(&mut model);

        let req = make_request_with_data(
            Operation::Create,
            EntityType::TimeEntry,
            json!({"task_id": task_id, "started_at": "2025-01-15T10:00:00Z", "duration_minutes": 90}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
    }

    #[test]
    fn test_time_entry_create_missing_task_id() {
        let mut model = Model::new();
        let req = make_request_with_data(
            Operation::Create,
            EntityType::TimeEntry,
            json!({"started_at": "2025-01-15T10:00:00Z"}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_DATA");
    }

    #[test]
    fn test_time_entry_create_task_not_found() {
        let mut model = Model::new();
        let req = make_request_with_data(
            Operation::Create,
            EntityType::TimeEntry,
            json!({"task_id": "00000000-0000-0000-0000-000000000000", "started_at": "2025-01-15T10:00:00Z"}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "NOT_FOUND");
    }

    #[test]
    fn test_time_entry_create_no_data() {
        let mut model = Model::new();
        let req = make_request(Operation::Create, EntityType::TimeEntry);
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_DATA");
    }

    #[test]
    fn test_time_entry_get() {
        let mut model = Model::new();
        let task_id = create_task_in_model(&mut model);

        let req = make_request_with_data(
            Operation::Create,
            EntityType::TimeEntry,
            json!({"task_id": task_id, "started_at": "2025-01-15T10:00:00Z"}),
        );
        let resp = super::super::process_request(&mut model, req);
        let entry_id = resp.data.unwrap()["id"].as_str().unwrap().to_string();

        let req = make_request_with_id(Operation::Get, EntityType::TimeEntry, &entry_id);
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
    }

    #[test]
    fn test_time_entry_get_not_found() {
        let mut model = Model::new();
        let req = make_request_with_id(
            Operation::Get,
            EntityType::TimeEntry,
            "00000000-0000-0000-0000-000000000000",
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "NOT_FOUND");
    }

    #[test]
    fn test_time_entry_get_missing_id() {
        let mut model = Model::new();
        let req = make_request(Operation::Get, EntityType::TimeEntry);
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_ID");
    }

    #[test]
    fn test_time_entry_update() {
        let mut model = Model::new();
        let task_id = create_task_in_model(&mut model);

        let req = make_request_with_data(
            Operation::Create,
            EntityType::TimeEntry,
            json!({"task_id": task_id, "started_at": "2025-01-15T10:00:00Z"}),
        );
        let resp = super::super::process_request(&mut model, req);
        let entry_id = resp.data.unwrap()["id"].as_str().unwrap().to_string();

        let req = make_request_with_id_and_data(
            Operation::Update,
            EntityType::TimeEntry,
            &entry_id,
            json!({
                "started_at": "2025-01-15T09:00:00Z",
                "ended_at": "2025-01-15T10:00:00Z",
                "duration_minutes": 60,
                "description": "updated"
            }),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success, "{:?}", resp.error);
    }

    #[test]
    fn test_time_entry_update_clear_fields() {
        let mut model = Model::new();
        let task_id = create_task_in_model(&mut model);

        let req = make_request_with_data(
            Operation::Create,
            EntityType::TimeEntry,
            json!({"task_id": task_id, "started_at": "2025-01-15T10:00:00Z", "ended_at": "2025-01-15T11:00:00Z"}),
        );
        let resp = super::super::process_request(&mut model, req);
        let entry_id = resp.data.unwrap()["id"].as_str().unwrap().to_string();

        let req = make_request_with_id_and_data(
            Operation::Update,
            EntityType::TimeEntry,
            &entry_id,
            json!({"ended_at": null, "duration_minutes": null, "description": null}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
    }

    #[test]
    fn test_time_entry_update_not_found() {
        let mut model = Model::new();
        let req = make_request_with_id_and_data(
            Operation::Update,
            EntityType::TimeEntry,
            "00000000-0000-0000-0000-000000000000",
            json!({"description": "x"}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "NOT_FOUND");
    }

    #[test]
    fn test_time_entry_delete() {
        let mut model = Model::new();
        let task_id = create_task_in_model(&mut model);

        let req = make_request_with_data(
            Operation::Create,
            EntityType::TimeEntry,
            json!({"task_id": task_id, "started_at": "2025-01-15T10:00:00Z"}),
        );
        let resp = super::super::process_request(&mut model, req);
        let entry_id = resp.data.unwrap()["id"].as_str().unwrap().to_string();

        let req = make_request_with_id(Operation::Delete, EntityType::TimeEntry, &entry_id);
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        assert_eq!(model.time_entries.len(), 0);
    }

    #[test]
    fn test_time_entry_delete_not_found() {
        let mut model = Model::new();
        let req = make_request_with_id(
            Operation::Delete,
            EntityType::TimeEntry,
            "00000000-0000-0000-0000-000000000000",
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "NOT_FOUND");
    }

    #[test]
    fn test_time_entry_update_missing_data() {
        let mut model = Model::new();
        let req = make_request_with_id(
            Operation::Update,
            EntityType::TimeEntry,
            "00000000-0000-0000-0000-000000000000",
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_DATA");
    }

    #[test]
    fn test_time_entry_list_filter_by_task() {
        let mut model = Model::new();
        let task_id = create_task_in_model(&mut model);
        let task_id2 = create_task_in_model(&mut model);

        super::super::process_request(
            &mut model,
            make_request_with_data(
                Operation::Create,
                EntityType::TimeEntry,
                json!({"task_id": task_id, "started_at": "2025-01-15T10:00:00Z"}),
            ),
        );
        super::super::process_request(
            &mut model,
            make_request_with_data(
                Operation::Create,
                EntityType::TimeEntry,
                json!({"task_id": task_id2, "started_at": "2025-01-15T11:00:00Z"}),
            ),
        );

        let req = PipeRequest {
            operation: Operation::List,
            entity: EntityType::TimeEntry,
            id: None,
            data: None,
            filters: Some(FilterParams {
                project_id: Some(task_id.clone()),
                ..Default::default()
            }),
        };
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        assert_eq!(resp.data.unwrap()["total"], 1);
    }

    // ========================================================================
    // WorkLog Handler Tests
    // ========================================================================

    #[test]
    fn test_work_log_list_empty() {
        let mut model = Model::new();
        let req = make_request(Operation::List, EntityType::WorkLog);
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        assert_eq!(resp.data.unwrap()["total"], 0);
    }

    #[test]
    fn test_work_log_create_and_list() {
        let mut model = Model::new();
        let task_id = create_task_in_model(&mut model);

        let req = make_request_with_data(
            Operation::Create,
            EntityType::WorkLog,
            json!({"task_id": task_id, "content": "Fixed bug in module X"}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success, "{:?}", resp.error);

        let req = make_request(Operation::List, EntityType::WorkLog);
        let resp = super::super::process_request(&mut model, req);
        assert_eq!(resp.data.unwrap()["total"], 1);
    }

    #[test]
    fn test_work_log_create_missing_task_id() {
        let mut model = Model::new();
        let req = make_request_with_data(
            Operation::Create,
            EntityType::WorkLog,
            json!({"content": "No task id"}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_DATA");
    }

    #[test]
    fn test_work_log_create_missing_content() {
        let mut model = Model::new();
        let task_id = create_task_in_model(&mut model);
        let req = make_request_with_data(
            Operation::Create,
            EntityType::WorkLog,
            json!({"task_id": task_id}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_DATA");
    }

    #[test]
    fn test_work_log_create_task_not_found() {
        let mut model = Model::new();
        let req = make_request_with_data(
            Operation::Create,
            EntityType::WorkLog,
            json!({"task_id": "00000000-0000-0000-0000-000000000000", "content": "test"}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "NOT_FOUND");
    }

    #[test]
    fn test_work_log_create_no_data() {
        let mut model = Model::new();
        let req = make_request(Operation::Create, EntityType::WorkLog);
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_DATA");
    }

    #[test]
    fn test_work_log_get() {
        let mut model = Model::new();
        let task_id = create_task_in_model(&mut model);

        let req = make_request_with_data(
            Operation::Create,
            EntityType::WorkLog,
            json!({"task_id": task_id, "content": "Work done"}),
        );
        let resp = super::super::process_request(&mut model, req);
        let log_id = resp.data.unwrap()["id"].as_str().unwrap().to_string();

        let req = make_request_with_id(Operation::Get, EntityType::WorkLog, &log_id);
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
    }

    #[test]
    fn test_work_log_get_not_found() {
        let mut model = Model::new();
        let req = make_request_with_id(
            Operation::Get,
            EntityType::WorkLog,
            "00000000-0000-0000-0000-000000000000",
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "NOT_FOUND");
    }

    #[test]
    fn test_work_log_get_missing_id() {
        let mut model = Model::new();
        let req = make_request(Operation::Get, EntityType::WorkLog);
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_ID");
    }

    #[test]
    fn test_work_log_update() {
        let mut model = Model::new();
        let task_id = create_task_in_model(&mut model);

        let req = make_request_with_data(
            Operation::Create,
            EntityType::WorkLog,
            json!({"task_id": task_id, "content": "Original"}),
        );
        let resp = super::super::process_request(&mut model, req);
        let log_id = resp.data.unwrap()["id"].as_str().unwrap().to_string();

        let req = make_request_with_id_and_data(
            Operation::Update,
            EntityType::WorkLog,
            &log_id,
            json!({"content": "Updated content"}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success, "{:?}", resp.error);
        assert_eq!(resp.data.unwrap()["content"], "Updated content");
    }

    #[test]
    fn test_work_log_update_not_found() {
        let mut model = Model::new();
        let req = make_request_with_id_and_data(
            Operation::Update,
            EntityType::WorkLog,
            "00000000-0000-0000-0000-000000000000",
            json!({"content": "x"}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "NOT_FOUND");
    }

    #[test]
    fn test_work_log_update_missing_id() {
        let mut model = Model::new();
        let req = make_request_with_data(
            Operation::Update,
            EntityType::WorkLog,
            json!({"content": "x"}),
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_ID");
    }

    #[test]
    fn test_work_log_delete() {
        let mut model = Model::new();
        let task_id = create_task_in_model(&mut model);

        let req = make_request_with_data(
            Operation::Create,
            EntityType::WorkLog,
            json!({"task_id": task_id, "content": "To delete"}),
        );
        let resp = super::super::process_request(&mut model, req);
        let log_id = resp.data.unwrap()["id"].as_str().unwrap().to_string();

        let req = make_request_with_id(Operation::Delete, EntityType::WorkLog, &log_id);
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        assert_eq!(model.work_logs.len(), 0);
    }

    #[test]
    fn test_work_log_delete_not_found() {
        let mut model = Model::new();
        let req = make_request_with_id(
            Operation::Delete,
            EntityType::WorkLog,
            "00000000-0000-0000-0000-000000000000",
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "NOT_FOUND");
    }

    #[test]
    fn test_work_log_delete_missing_id() {
        let mut model = Model::new();
        let req = make_request(Operation::Delete, EntityType::WorkLog);
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_ID");
    }

    #[test]
    fn test_work_log_update_missing_data() {
        let mut model = Model::new();
        let req = make_request_with_id(
            Operation::Update,
            EntityType::WorkLog,
            "00000000-0000-0000-0000-000000000000",
        );
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_DATA");
    }

    #[test]
    fn test_work_log_list_filter_by_task() {
        let mut model = Model::new();
        let task_id = create_task_in_model(&mut model);
        let task_id2 = create_task_in_model(&mut model);

        super::super::process_request(
            &mut model,
            make_request_with_data(
                Operation::Create,
                EntityType::WorkLog,
                json!({"task_id": task_id, "content": "log 1"}),
            ),
        );
        super::super::process_request(
            &mut model,
            make_request_with_data(
                Operation::Create,
                EntityType::WorkLog,
                json!({"task_id": task_id2, "content": "log 2"}),
            ),
        );

        let req = PipeRequest {
            operation: Operation::List,
            entity: EntityType::WorkLog,
            id: None,
            data: None,
            filters: Some(FilterParams {
                project_id: Some(task_id.clone()),
                ..Default::default()
            }),
        };
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        assert_eq!(resp.data.unwrap()["total"], 1);
    }

    #[test]
    fn test_work_log_list_filter_by_search() {
        let mut model = Model::new();
        let task_id = create_task_in_model(&mut model);

        super::super::process_request(
            &mut model,
            make_request_with_data(
                Operation::Create,
                EntityType::WorkLog,
                json!({"task_id": task_id, "content": "Fixed authentication bug"}),
            ),
        );
        super::super::process_request(
            &mut model,
            make_request_with_data(
                Operation::Create,
                EntityType::WorkLog,
                json!({"task_id": task_id, "content": "Refactored database module"}),
            ),
        );

        let req = PipeRequest {
            operation: Operation::List,
            entity: EntityType::WorkLog,
            id: None,
            data: None,
            filters: Some(FilterParams {
                search: Some("auth".to_string()),
                ..Default::default()
            }),
        };
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        assert_eq!(resp.data.unwrap()["total"], 1);
    }
}
