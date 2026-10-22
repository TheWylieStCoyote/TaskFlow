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
    use chrono::{Duration, TimeZone, Utc};
    use serde_json::json;
    use taskflow::app::Model;
    use taskflow::domain::{
        today, AuditAction, AuditLogEntry, AuditLogEntryId, FieldChange, Filter, GoalStatus,
        HabitFrequency, Priority, ProjectId, Quarter, SavedFilter, SortSpec, Task, TaskId,
        TaskStatus,
    };
    use types::*;
    use uuid::Uuid;

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
            "audit_log",
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

    // ========================================================================
    // Task List Filter Tests
    // ========================================================================

    const NIL_UUID: &str = "00000000-0000-0000-0000-000000000000";

    fn process(model: &mut Model, req: PipeRequest) -> PipeResponse<serde_json::Value> {
        super::super::process_request(model, req)
    }

    fn list_tasks_with(model: &mut Model, filters: FilterParams) -> serde_json::Value {
        let req = PipeRequest {
            operation: Operation::List,
            entity: EntityType::Task,
            id: None,
            data: None,
            filters: Some(filters),
        };
        let resp = process(model, req);
        assert!(resp.success, "{:?}", resp.error);
        resp.data.unwrap()
    }

    fn titles(data: &serde_json::Value) -> Vec<String> {
        data["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["title"].as_str().unwrap().to_string())
            .collect()
    }

    fn sorted_titles(data: &serde_json::Value) -> Vec<String> {
        let mut t = titles(data);
        t.sort();
        t
    }

    fn at(year: i32, month: u32, day: u32) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(year, month, day, 12, 0, 0).unwrap()
    }

    /// Five tasks with distinct statuses, priorities, tags, dates and projects.
    ///
    /// | title           | status      | priority | tags              | due        | project |
    /// |-----------------|-------------|----------|-------------------|------------|---------|
    /// | Alpha report    | in_progress | high     | Work, quarterly   | today - 1  | yes     |
    /// | Bravo chores    | todo        | low      | home              | today      | no      |
    /// | Charlie blocked | blocked     | urgent   | work, home        | today + 1  | yes     |
    /// | Delta done      | done        | medium   |                   | none       | no      |
    /// | Echo cancelled  | cancelled   | none     |                   | none       | no      |
    fn create_filter_model() -> (Model, ProjectId) {
        let mut model = Model::new();
        let project_id = ProjectId(Uuid::new_v4());
        let today = today();

        let mut alpha = Task::new("Alpha report")
            .with_priority(Priority::High)
            .with_tags(vec!["Work".to_string(), "quarterly".to_string()]);
        alpha.status = TaskStatus::InProgress;
        alpha.project_id = Some(project_id);
        alpha.due_date = Some(today - Duration::days(1));
        alpha.created_at = at(2030, 1, 5);
        alpha.updated_at = at(2030, 2, 1);

        let mut bravo = Task::new("Bravo chores")
            .with_priority(Priority::Low)
            .with_tags(vec!["home".to_string()])
            .with_description("Clean the kitchen");
        bravo.due_date = Some(today);
        bravo.created_at = at(2030, 1, 1);
        bravo.updated_at = at(2030, 2, 3);

        let mut charlie = Task::new("Charlie blocked")
            .with_priority(Priority::Urgent)
            .with_tags(vec!["work".to_string(), "home".to_string()]);
        charlie.status = TaskStatus::Blocked;
        charlie.project_id = Some(project_id);
        charlie.due_date = Some(today + Duration::days(1));
        charlie.created_at = at(2030, 1, 3);
        charlie.updated_at = at(2030, 2, 2);

        let mut delta = Task::new("Delta done").with_priority(Priority::Medium);
        delta.status = TaskStatus::Done;
        delta.created_at = at(2030, 1, 2);
        delta.updated_at = at(2030, 2, 4);

        let mut echo = Task::new("Echo cancelled");
        echo.status = TaskStatus::Cancelled;
        echo.created_at = at(2030, 1, 4);
        echo.updated_at = at(2030, 2, 5);

        for task in [alpha, bravo, charlie, delta, echo] {
            model.tasks.insert(task.id, task);
        }
        model.refresh_visible_tasks();
        (model, project_id)
    }

    fn strings(items: &[&str]) -> Option<Vec<String>> {
        Some(items.iter().map(|s| (*s).to_string()).collect())
    }

    #[test]
    fn test_task_list_without_filters_includes_completed() {
        let (mut model, _) = create_filter_model();
        let resp = process(&mut model, make_request(Operation::List, EntityType::Task));
        assert!(resp.success);
        let data = resp.data.unwrap();
        assert_eq!(data["total"], 5);
        assert_eq!(data["offset"], 0);
        assert_eq!(data["limit"], 100);
        assert_eq!(data["tasks"].as_array().unwrap().len(), 5);
        let meta = resp.metadata.unwrap();
        assert_eq!(meta.total, Some(5));
        assert_eq!(meta.offset, Some(0));
        assert_eq!(meta.limit, Some(100));
    }

    #[test]
    fn test_task_list_default_filters_exclude_completed() {
        let (mut model, _) = create_filter_model();
        let data = list_tasks_with(&mut model, FilterParams::default());
        assert_eq!(
            sorted_titles(&data),
            ["Alpha report", "Bravo chores", "Charlie blocked"]
        );

        let data = list_tasks_with(
            &mut model,
            FilterParams {
                include_completed: Some(true),
                ..Default::default()
            },
        );
        assert_eq!(data["total"], 5);
    }

    #[test]
    fn test_task_list_filter_by_project() {
        let (mut model, project_id) = create_filter_model();
        let data = list_tasks_with(
            &mut model,
            FilterParams {
                project_id: Some(project_id.0.to_string()),
                ..Default::default()
            },
        );
        assert_eq!(sorted_titles(&data), ["Alpha report", "Charlie blocked"]);

        // A project id that is not a UUID is ignored rather than matching nothing.
        let data = list_tasks_with(
            &mut model,
            FilterParams {
                project_id: Some("not-a-uuid".to_string()),
                ..Default::default()
            },
        );
        assert_eq!(data["total"], 3);

        // A valid but unknown project matches nothing.
        let data = list_tasks_with(
            &mut model,
            FilterParams {
                project_id: Some(NIL_UUID.to_string()),
                ..Default::default()
            },
        );
        assert_eq!(data["total"], 0);
    }

    #[test]
    fn test_task_list_filter_by_tags_all_and_any() {
        let (mut model, _) = create_filter_model();

        // Default mode is "all".
        let data = list_tasks_with(
            &mut model,
            FilterParams {
                tags: strings(&["work", "home"]),
                ..Default::default()
            },
        );
        assert_eq!(titles(&data), ["Charlie blocked"]);

        let data = list_tasks_with(
            &mut model,
            FilterParams {
                tags: strings(&["work", "home"]),
                tags_mode: Some("any".to_string()),
                ..Default::default()
            },
        );
        assert_eq!(
            sorted_titles(&data),
            ["Alpha report", "Bravo chores", "Charlie blocked"]
        );

        // Matching is case-insensitive on both sides.
        let data = list_tasks_with(
            &mut model,
            FilterParams {
                tags: strings(&["WORK"]),
                tags_mode: Some("all".to_string()),
                ..Default::default()
            },
        );
        assert_eq!(sorted_titles(&data), ["Alpha report", "Charlie blocked"]);
    }

    #[test]
    fn test_task_list_filter_by_status() {
        let (mut model, _) = create_filter_model();

        let data = list_tasks_with(
            &mut model,
            FilterParams {
                status: strings(&["in_progress", "blocked"]),
                ..Default::default()
            },
        );
        assert_eq!(sorted_titles(&data), ["Alpha report", "Charlie blocked"]);

        // Status filter runs before the completion filter, so "done" alone
        // yields nothing unless completed tasks are included.
        let data = list_tasks_with(
            &mut model,
            FilterParams {
                status: strings(&["done"]),
                ..Default::default()
            },
        );
        assert_eq!(data["total"], 0);

        let data = list_tasks_with(
            &mut model,
            FilterParams {
                status: strings(&["done"]),
                include_completed: Some(true),
                ..Default::default()
            },
        );
        assert_eq!(titles(&data), ["Delta done"]);

        // Spelling variants accepted by parse_status.
        let data = list_tasks_with(
            &mut model,
            FilterParams {
                status: strings(&["In-Progress", "COMPLETED", "canceled"]),
                include_completed: Some(true),
                ..Default::default()
            },
        );
        assert_eq!(
            sorted_titles(&data),
            ["Alpha report", "Delta done", "Echo cancelled"]
        );

        // Only unknown statuses: filter is skipped entirely.
        let data = list_tasks_with(
            &mut model,
            FilterParams {
                status: strings(&["bogus"]),
                ..Default::default()
            },
        );
        assert_eq!(data["total"], 3);
    }

    #[test]
    fn test_task_list_filter_by_priority() {
        let (mut model, _) = create_filter_model();

        let data = list_tasks_with(
            &mut model,
            FilterParams {
                priority: strings(&["high", "URGENT"]),
                ..Default::default()
            },
        );
        assert_eq!(sorted_titles(&data), ["Alpha report", "Charlie blocked"]);

        let data = list_tasks_with(
            &mut model,
            FilterParams {
                priority: strings(&["med"]),
                include_completed: Some(true),
                ..Default::default()
            },
        );
        assert_eq!(titles(&data), ["Delta done"]);

        let data = list_tasks_with(
            &mut model,
            FilterParams {
                priority: strings(&["none", "low"]),
                include_completed: Some(true),
                ..Default::default()
            },
        );
        assert_eq!(sorted_titles(&data), ["Bravo chores", "Echo cancelled"]);

        let data = list_tasks_with(
            &mut model,
            FilterParams {
                priority: strings(&["bogus"]),
                ..Default::default()
            },
        );
        assert_eq!(data["total"], 3);
    }

    #[test]
    fn test_task_list_filter_by_search() {
        let (mut model, _) = create_filter_model();
        let search = |model: &mut Model, text: &str| {
            list_tasks_with(
                model,
                FilterParams {
                    search: Some(text.to_string()),
                    ..Default::default()
                },
            )
        };

        // Title match, case-insensitive.
        assert_eq!(titles(&search(&mut model, "ALPHA")), ["Alpha report"]);
        // Description match.
        assert_eq!(titles(&search(&mut model, "kitchen")), ["Bravo chores"]);
        // Tag match.
        assert_eq!(titles(&search(&mut model, "quarterly")), ["Alpha report"]);
        // No match.
        assert_eq!(search(&mut model, "zzz")["total"], 0);
    }

    #[test]
    fn test_task_list_filter_by_due_dates() {
        let (mut model, _) = create_filter_model();
        let today = today().format("%Y-%m-%d").to_string();

        // due_before is inclusive.
        let data = list_tasks_with(
            &mut model,
            FilterParams {
                due_before: Some(today.clone()),
                include_completed: Some(true),
                ..Default::default()
            },
        );
        assert_eq!(sorted_titles(&data), ["Alpha report", "Bravo chores"]);

        // due_after is inclusive.
        let data = list_tasks_with(
            &mut model,
            FilterParams {
                due_after: Some(today.clone()),
                ..Default::default()
            },
        );
        assert_eq!(sorted_titles(&data), ["Bravo chores", "Charlie blocked"]);

        // Both bounds on the same day.
        let data = list_tasks_with(
            &mut model,
            FilterParams {
                due_before: Some(today.clone()),
                due_after: Some(today),
                ..Default::default()
            },
        );
        assert_eq!(titles(&data), ["Bravo chores"]);

        // Unparseable dates are ignored.
        let data = list_tasks_with(
            &mut model,
            FilterParams {
                due_before: Some("31/12/2030".to_string()),
                due_after: Some("soon".to_string()),
                ..Default::default()
            },
        );
        assert_eq!(data["total"], 3);
    }

    fn sort_tasks(
        model: &mut Model,
        sort_by: &str,
        sort_order: Option<&str>,
        include_completed: bool,
    ) -> Vec<String> {
        titles(&list_tasks_with(
            model,
            FilterParams {
                sort_by: Some(sort_by.to_string()),
                sort_order: sort_order.map(str::to_string),
                include_completed: Some(include_completed),
                ..Default::default()
            },
        ))
    }

    #[test]
    fn test_task_list_sort_by_due_date() {
        let (mut model, _) = create_filter_model();
        assert_eq!(
            sort_tasks(&mut model, "due_date", None, false),
            ["Alpha report", "Bravo chores", "Charlie blocked"]
        );
        assert_eq!(
            sort_tasks(&mut model, "due_date", Some("desc"), false),
            ["Charlie blocked", "Bravo chores", "Alpha report"]
        );
    }

    #[test]
    fn test_task_list_sort_by_priority() {
        let (mut model, _) = create_filter_model();
        // Default order puts the highest priority first.
        assert_eq!(
            sort_tasks(&mut model, "priority", Some("asc"), false),
            ["Charlie blocked", "Alpha report", "Bravo chores"]
        );
        assert_eq!(
            sort_tasks(&mut model, "priority", Some("desc"), false),
            ["Bravo chores", "Alpha report", "Charlie blocked"]
        );
        // All five priority levels participate in the ordering.
        assert_eq!(
            sort_tasks(&mut model, "priority", None, true),
            [
                "Charlie blocked",
                "Alpha report",
                "Delta done",
                "Bravo chores",
                "Echo cancelled"
            ]
        );
    }

    #[test]
    fn test_task_list_sort_by_title() {
        let (mut model, _) = create_filter_model();
        assert_eq!(
            sort_tasks(&mut model, "title", None, true),
            [
                "Alpha report",
                "Bravo chores",
                "Charlie blocked",
                "Delta done",
                "Echo cancelled"
            ]
        );
        assert_eq!(
            sort_tasks(&mut model, "title", Some("desc"), false),
            ["Charlie blocked", "Bravo chores", "Alpha report"]
        );
    }

    #[test]
    fn test_task_list_sort_by_created_and_updated() {
        let (mut model, _) = create_filter_model();
        assert_eq!(
            sort_tasks(&mut model, "created", None, false),
            ["Bravo chores", "Charlie blocked", "Alpha report"]
        );
        assert_eq!(
            sort_tasks(&mut model, "created_at", Some("desc"), false),
            ["Alpha report", "Charlie blocked", "Bravo chores"]
        );
        assert_eq!(
            sort_tasks(&mut model, "updated", None, false),
            ["Alpha report", "Charlie blocked", "Bravo chores"]
        );
        assert_eq!(
            sort_tasks(&mut model, "updated_at", Some("desc"), false),
            ["Bravo chores", "Charlie blocked", "Alpha report"]
        );
    }

    #[test]
    fn test_task_list_unknown_sort_key_is_ignored() {
        let (mut model, _) = create_filter_model();
        let data = list_tasks_with(
            &mut model,
            FilterParams {
                sort_by: Some("bogus".to_string()),
                ..Default::default()
            },
        );
        assert_eq!(data["total"], 3);
        assert_eq!(data["tasks"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_task_list_pagination() {
        let (mut model, _) = create_filter_model();
        let page = |model: &mut Model, offset: usize, limit: usize| {
            let req = PipeRequest {
                operation: Operation::List,
                entity: EntityType::Task,
                id: None,
                data: None,
                filters: Some(FilterParams {
                    sort_by: Some("title".to_string()),
                    include_completed: Some(true),
                    offset: Some(offset),
                    limit: Some(limit),
                    ..Default::default()
                }),
            };
            process(model, req)
        };

        let resp = page(&mut model, 0, 2);
        let data = resp.data.as_ref().unwrap();
        assert_eq!(titles(data), ["Alpha report", "Bravo chores"]);
        assert_eq!(data["total"], 5);
        assert_eq!(data["offset"], 0);
        assert_eq!(data["limit"], 2);

        let resp = page(&mut model, 2, 2);
        let data = resp.data.as_ref().unwrap();
        assert_eq!(titles(data), ["Charlie blocked", "Delta done"]);
        let meta = resp.metadata.unwrap();
        assert_eq!(
            (meta.total, meta.offset, meta.limit),
            (Some(5), Some(2), Some(2))
        );

        let resp = page(&mut model, 10, 2);
        let data = resp.data.unwrap();
        assert!(titles(&data).is_empty());
        assert_eq!(data["total"], 5);
    }

    // ========================================================================
    // Task Create Tests
    // ========================================================================

    fn create_task_resp(
        model: &mut Model,
        data: serde_json::Value,
    ) -> PipeResponse<serde_json::Value> {
        process(
            model,
            make_request_with_data(Operation::Create, EntityType::Task, data),
        )
    }

    fn task_by_id(model: &Model, id: &str) -> Task {
        let uuid = Uuid::parse_str(id).unwrap();
        model.tasks[&taskflow::domain::TaskId(uuid)].clone()
    }

    #[test]
    fn test_task_create_with_all_fields() {
        let mut model = Model::new();
        let project = Uuid::new_v4();
        let dep = Uuid::new_v4();
        let resp = create_task_resp(
            &mut model,
            json!({
                "title": "Full task",
                "description": "Details",
                "status": "in_progress",
                "priority": "urgent",
                "project_id": project.to_string(),
                "tags": ["a", "b"],
                "due_date": "2030-03-04",
                "scheduled_date": "2030-03-01",
                "estimated_minutes": 90,
                "dependencies": [dep.to_string(), "not-a-uuid"]
            }),
        );
        assert!(resp.success, "{:?}", resp.error);
        let data = resp.data.unwrap();
        assert_eq!(data["title"], "Full task");
        assert_eq!(data["status"], "inprogress");

        let task = task_by_id(&model, data["id"].as_str().unwrap());
        assert_eq!(task.description.as_deref(), Some("Details"));
        assert_eq!(task.status, TaskStatus::InProgress);
        assert!(task.completed_at.is_none());
        assert_eq!(task.priority, Priority::Urgent);
        assert_eq!(task.project_id, Some(ProjectId(project)));
        assert_eq!(task.tags, ["a", "b"]);
        assert_eq!(task.due_date.unwrap().to_string(), "2030-03-04");
        assert_eq!(task.scheduled_date.unwrap().to_string(), "2030-03-01");
        assert_eq!(task.estimated_minutes, Some(90));
        assert_eq!(task.dependencies, [taskflow::domain::TaskId(dep)]);
    }

    #[test]
    fn test_task_create_complete_status_sets_completed_at() {
        let mut model = Model::new();
        for status in ["done", "cancelled"] {
            let resp = create_task_resp(&mut model, json!({"title": status, "status": status}));
            let data = resp.data.unwrap();
            let task = task_by_id(&model, data["id"].as_str().unwrap());
            assert!(task.status.is_complete(), "{status}");
            assert!(task.completed_at.is_some(), "{status}");
        }
    }

    #[test]
    fn test_task_create_ignores_unparseable_values() {
        let mut model = Model::new();
        let resp = create_task_resp(
            &mut model,
            json!({
                "title": "Loose task",
                "status": "someday",
                "priority": "whenever",
                "project_id": "not-a-uuid",
                "due_date": "31/12/2030",
                "scheduled_date": "tomorrow",
                "dependencies": ["nope"]
            }),
        );
        assert!(resp.success, "{:?}", resp.error);
        let task = task_by_id(&model, resp.data.unwrap()["id"].as_str().unwrap());
        assert_eq!(task.status, TaskStatus::Todo);
        assert_eq!(task.priority, Priority::None);
        assert!(task.project_id.is_none());
        assert!(task.due_date.is_none());
        assert!(task.scheduled_date.is_none());
        assert!(task.dependencies.is_empty());
    }

    #[test]
    fn test_task_create_rejects_bad_input() {
        let mut model = Model::new();

        let resp = process(
            &mut model,
            make_request(Operation::Create, EntityType::Task),
        );
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_DATA");

        let resp = create_task_resp(&mut model, json!({"description": "no title"}));
        assert!(!resp.success);
        let err = resp.error.unwrap();
        assert_eq!(err.code, "MISSING_DATA");
        assert_eq!(err.message, "Task title is required");

        let resp = create_task_resp(&mut model, json!({"title": 123}));
        assert_eq!(resp.error.as_ref().unwrap().code, "INVALID_DATA");

        let resp = create_task_resp(
            &mut model,
            json!({"title": "x", "estimated_minutes": "lots"}),
        );
        let err = resp.error.unwrap();
        assert_eq!(err.code, "INVALID_DATA");
        assert!(err.message.starts_with("Failed to parse task data:"));

        assert!(model.tasks.is_empty());
    }

    // ========================================================================
    // Task Update Tests
    // ========================================================================

    fn update_task_resp(
        model: &mut Model,
        id: &str,
        data: serde_json::Value,
    ) -> PipeResponse<serde_json::Value> {
        process(
            model,
            make_request_with_id_and_data(Operation::Update, EntityType::Task, id, data),
        )
    }

    fn seed_task(model: &mut Model) -> String {
        let data = create_task_resp(model, json!({"title": "Seed"}))
            .data
            .unwrap();
        data["id"].as_str().unwrap().to_string()
    }

    #[test]
    fn test_task_update_error_paths() {
        let mut model = Model::new();
        let id = seed_task(&mut model);

        let resp = process(
            &mut model,
            make_request_with_data(Operation::Update, EntityType::Task, json!({"title": "x"})),
        );
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_ID");

        let resp = process(
            &mut model,
            make_request_with_id(Operation::Update, EntityType::Task, &id),
        );
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_DATA");

        let resp = update_task_resp(&mut model, "garbage", json!({"title": "x"}));
        let err = resp.error.unwrap();
        assert_eq!(err.code, "INVALID_ID");
        assert_eq!(err.message, "Invalid UUID format: garbage");

        let resp = update_task_resp(&mut model, NIL_UUID, json!({"title": "x"}));
        let err = resp.error.unwrap();
        assert_eq!(err.code, "NOT_FOUND");
        assert_eq!(err.message, format!("Task not found: {NIL_UUID}"));

        assert_eq!(task_by_id(&model, &id).title, "Seed");
    }

    #[test]
    fn test_task_update_title_and_description() {
        let mut model = Model::new();
        let id = seed_task(&mut model);

        let resp = update_task_resp(
            &mut model,
            &id,
            json!({"title": "Renamed", "description": "Now with details"}),
        );
        assert!(resp.success, "{:?}", resp.error);
        assert_eq!(resp.data.unwrap()["title"], "Renamed");
        let task = task_by_id(&model, &id);
        assert_eq!(task.title, "Renamed");
        assert_eq!(task.description.as_deref(), Some("Now with details"));

        // Non-string title is ignored; null description clears it.
        update_task_resp(&mut model, &id, json!({"title": 7, "description": null}));
        let task = task_by_id(&model, &id);
        assert_eq!(task.title, "Renamed");
        assert!(task.description.is_none());

        // Non-string, non-null description also clears it.
        update_task_resp(&mut model, &id, json!({"description": "x"}));
        update_task_resp(&mut model, &id, json!({"description": 42}));
        assert!(task_by_id(&model, &id).description.is_none());
    }

    #[test]
    fn test_task_update_status_transitions() {
        let mut model = Model::new();
        let id = seed_task(&mut model);

        update_task_resp(&mut model, &id, json!({"status": "done"}));
        let task = task_by_id(&model, &id);
        assert_eq!(task.status, TaskStatus::Done);
        let completed_at = task.completed_at.expect("completed_at set on completion");

        // Completing an already complete task keeps the original timestamp.
        update_task_resp(&mut model, &id, json!({"status": "cancelled"}));
        let task = task_by_id(&model, &id);
        assert_eq!(task.status, TaskStatus::Cancelled);
        assert_eq!(task.completed_at, Some(completed_at));

        // Unknown status leaves everything untouched.
        update_task_resp(&mut model, &id, json!({"status": "bogus"}));
        assert_eq!(task_by_id(&model, &id).status, TaskStatus::Cancelled);

        // Reopening clears completed_at.
        update_task_resp(&mut model, &id, json!({"status": "in-progress"}));
        let task = task_by_id(&model, &id);
        assert_eq!(task.status, TaskStatus::InProgress);
        assert!(task.completed_at.is_none());

        // Moving between two open statuses never touches completed_at.
        update_task_resp(&mut model, &id, json!({"status": "blocked"}));
        let task = task_by_id(&model, &id);
        assert_eq!(task.status, TaskStatus::Blocked);
        assert!(task.completed_at.is_none());
    }

    #[test]
    fn test_task_update_priority_project_and_tags() {
        let mut model = Model::new();
        let id = seed_task(&mut model);
        let project = Uuid::new_v4();

        update_task_resp(
            &mut model,
            &id,
            json!({
                "priority": "med",
                "project_id": project.to_string(),
                "tags": ["a", 1, "b", null]
            }),
        );
        let task = task_by_id(&model, &id);
        assert_eq!(task.priority, Priority::Medium);
        assert_eq!(task.project_id, Some(ProjectId(project)));
        assert_eq!(task.tags, ["a", "b"]);

        // Unknown priority and non-array tags are ignored; bad project id clears.
        update_task_resp(
            &mut model,
            &id,
            json!({"priority": "bogus", "project_id": "not-a-uuid", "tags": "a,b"}),
        );
        let task = task_by_id(&model, &id);
        assert_eq!(task.priority, Priority::Medium);
        assert!(task.project_id.is_none());
        assert_eq!(task.tags, ["a", "b"]);

        update_task_resp(&mut model, &id, json!({"project_id": project.to_string()}));
        update_task_resp(&mut model, &id, json!({"project_id": null, "tags": []}));
        let task = task_by_id(&model, &id);
        assert!(task.project_id.is_none());
        assert!(task.tags.is_empty());
    }

    #[test]
    fn test_task_update_dates_estimate_and_dependencies() {
        let mut model = Model::new();
        let id = seed_task(&mut model);
        let dep = Uuid::new_v4();

        update_task_resp(
            &mut model,
            &id,
            json!({
                "due_date": "2030-05-06",
                "scheduled_date": "2030-05-01",
                "estimated_minutes": 45,
                "dependencies": [dep.to_string(), "bad", 3]
            }),
        );
        let task = task_by_id(&model, &id);
        assert_eq!(task.due_date.unwrap().to_string(), "2030-05-06");
        assert_eq!(task.scheduled_date.unwrap().to_string(), "2030-05-01");
        assert_eq!(task.estimated_minutes, Some(45));
        assert_eq!(task.dependencies, [taskflow::domain::TaskId(dep)]);

        // Nulls clear the optional fields.
        update_task_resp(
            &mut model,
            &id,
            json!({"due_date": null, "scheduled_date": null, "estimated_minutes": null}),
        );
        let task = task_by_id(&model, &id);
        assert!(task.due_date.is_none());
        assert!(task.scheduled_date.is_none());
        assert!(task.estimated_minutes.is_none());

        // Unparseable values also resolve to None.
        update_task_resp(&mut model, &id, json!({"due_date": "2030-05-06"}));
        update_task_resp(
            &mut model,
            &id,
            json!({
                "due_date": "06/05/2030",
                "scheduled_date": 20300501,
                "estimated_minutes": "many",
                "dependencies": []
            }),
        );
        let task = task_by_id(&model, &id);
        assert!(task.due_date.is_none());
        assert!(task.scheduled_date.is_none());
        assert!(task.estimated_minutes.is_none());
        assert!(task.dependencies.is_empty());
    }

    #[test]
    fn test_task_update_bumps_updated_at_and_returns_task() {
        let mut model = Model::new();
        let id = seed_task(&mut model);
        let before = at(2000, 1, 1);
        model
            .tasks
            .get_mut(&taskflow::domain::TaskId(Uuid::parse_str(&id).unwrap()))
            .unwrap()
            .updated_at = before;

        let resp = update_task_resp(&mut model, &id, json!({"title": "Bumped"}));
        let data = resp.data.unwrap();
        assert_eq!(data["id"], id);
        assert_eq!(data["title"], "Bumped");
        assert!(task_by_id(&model, &id).updated_at > before);
    }

    // ========================================================================
    // Goal Update / KeyResult List Tests
    // ========================================================================

    fn goal_by_id(model: &Model, id: &str) -> taskflow::domain::Goal {
        let uuid = Uuid::parse_str(id).unwrap();
        model.goals[&taskflow::domain::GoalId(uuid)].clone()
    }

    fn update_goal_resp(
        model: &mut Model,
        id: &str,
        data: serde_json::Value,
    ) -> PipeResponse<serde_json::Value> {
        process(
            model,
            make_request_with_id_and_data(Operation::Update, EntityType::Goal, id, data),
        )
    }

    #[test]
    fn test_goal_update_error_paths() {
        let mut model = Model::new();
        let id = create_goal_in_model(&mut model);

        let resp = process(
            &mut model,
            make_request_with_id(Operation::Update, EntityType::Goal, &id),
        );
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_DATA");

        let resp = update_goal_resp(&mut model, "nope", json!({"name": "x"}));
        let err = resp.error.unwrap();
        assert_eq!(err.code, "INVALID_ID");
        assert_eq!(err.message, "Invalid UUID format: nope");
    }

    #[test]
    fn test_goal_update_dates_and_quarter() {
        let mut model = Model::new();
        let id = create_goal_in_model(&mut model);

        let resp = update_goal_resp(
            &mut model,
            &id,
            json!({
                "start_date": "2030-01-01",
                "due_date": "2030-03-31",
                "quarter": {"year": 2030, "quarter": "q1"}
            }),
        );
        assert!(resp.success, "{:?}", resp.error);
        let goal = goal_by_id(&model, &id);
        assert_eq!(goal.start_date.unwrap().to_string(), "2030-01-01");
        assert_eq!(goal.due_date.unwrap().to_string(), "2030-03-31");
        assert_eq!(goal.quarter, Some((2030, Quarter::Q1)));

        // Unknown quarter or missing year leaves the quarter unchanged.
        update_goal_resp(
            &mut model,
            &id,
            json!({"quarter": {"year": 2031, "quarter": "Q9"}}),
        );
        update_goal_resp(&mut model, &id, json!({"quarter": {"quarter": "Q2"}}));
        assert_eq!(goal_by_id(&model, &id).quarter, Some((2030, Quarter::Q1)));

        for (q_str, q) in [
            ("Q2", Quarter::Q2),
            ("q3", Quarter::Q3),
            ("Q4", Quarter::Q4),
        ] {
            update_goal_resp(
                &mut model,
                &id,
                json!({"quarter": {"year": 2031, "quarter": q_str}}),
            );
            assert_eq!(goal_by_id(&model, &id).quarter, Some((2031, q)));
        }

        // Invalid dates resolve to None; nulls clear.
        update_goal_resp(
            &mut model,
            &id,
            json!({"start_date": "Jan 1", "due_date": null}),
        );
        let goal = goal_by_id(&model, &id);
        assert!(goal.start_date.is_none());
        assert!(goal.due_date.is_none());

        update_goal_resp(
            &mut model,
            &id,
            json!({"start_date": null, "quarter": null}),
        );
        let goal = goal_by_id(&model, &id);
        assert!(goal.start_date.is_none());
        assert!(goal.quarter.is_none());
    }

    #[test]
    fn test_goal_update_status_progress_color_icon() {
        let mut model = Model::new();
        let id = create_goal_in_model(&mut model);

        update_goal_resp(
            &mut model,
            &id,
            json!({"manual_progress": 150, "color": "#ff0000", "icon": "target", "description": "d"}),
        );
        let goal = goal_by_id(&model, &id);
        assert_eq!(
            goal.manual_progress,
            Some(100),
            "progress is clamped to 100"
        );
        assert_eq!(goal.color.as_deref(), Some("#ff0000"));
        assert_eq!(goal.icon.as_deref(), Some("target"));
        assert_eq!(goal.description.as_deref(), Some("d"));

        update_goal_resp(
            &mut model,
            &id,
            json!({"manual_progress": 42, "status": "bogus", "description": null}),
        );
        let goal = goal_by_id(&model, &id);
        assert_eq!(goal.manual_progress, Some(42));
        assert_eq!(goal.status, GoalStatus::Active, "unknown status is ignored");
        assert!(goal.description.is_none());

        for (s, expected) in [
            ("hold", GoalStatus::OnHold),
            ("done", GoalStatus::Completed),
            ("ARCHIVED", GoalStatus::Archived),
            ("active", GoalStatus::Active),
            ("on-hold", GoalStatus::OnHold),
        ] {
            update_goal_resp(&mut model, &id, json!({"status": s}));
            assert_eq!(goal_by_id(&model, &id).status, expected, "{s}");
        }

        update_goal_resp(
            &mut model,
            &id,
            json!({"manual_progress": null, "color": null, "icon": null}),
        );
        let goal = goal_by_id(&model, &id);
        assert!(goal.manual_progress.is_none());
        assert!(goal.color.is_none());
        assert!(goal.icon.is_none());
    }

    fn create_kr(model: &mut Model, goal_id: &str, name: &str) -> String {
        let resp = process(
            model,
            make_request_with_data(
                Operation::Create,
                EntityType::KeyResult,
                json!({"goal_id": goal_id, "name": name, "target_value": 10.0}),
            ),
        );
        assert!(resp.success, "{:?}", resp.error);
        resp.data.unwrap()["id"].as_str().unwrap().to_string()
    }

    fn list_krs(model: &mut Model, filters: Option<FilterParams>) -> serde_json::Value {
        let req = PipeRequest {
            operation: Operation::List,
            entity: EntityType::KeyResult,
            id: None,
            data: None,
            filters,
        };
        let resp = process(model, req);
        assert!(resp.success, "{:?}", resp.error);
        resp.data.unwrap()
    }

    fn kr_names(data: &serde_json::Value) -> Vec<String> {
        data["key_results"]
            .as_array()
            .unwrap()
            .iter()
            .map(|kr| kr["name"].as_str().unwrap().to_string())
            .collect()
    }

    #[test]
    fn test_key_result_list_filters_and_sorting() {
        let mut model = Model::new();
        let goal1 = create_goal_in_model(&mut model);
        let goal2 = create_goal_in_model(&mut model);
        create_kr(&mut model, &goal1, "Zeta metric");
        create_kr(&mut model, &goal1, "Alpha metric");
        let beta = create_kr(&mut model, &goal2, "Beta metric");
        process(
            &mut model,
            make_request_with_id_and_data(
                Operation::Update,
                EntityType::KeyResult,
                &beta,
                json!({"description": "User retention"}),
            ),
        );

        // No filters: everything, unsorted.
        let data = list_krs(&mut model, None);
        assert_eq!(data["total"], 3);

        // Filter by goal (via project_id) sorts by name.
        let data = list_krs(
            &mut model,
            Some(FilterParams {
                project_id: Some(goal1.clone()),
                ..Default::default()
            }),
        );
        assert_eq!(kr_names(&data), ["Alpha metric", "Zeta metric"]);

        // Non-UUID goal filter is ignored.
        let data = list_krs(
            &mut model,
            Some(FilterParams {
                project_id: Some("not-a-uuid".to_string()),
                ..Default::default()
            }),
        );
        assert_eq!(
            kr_names(&data),
            ["Alpha metric", "Beta metric", "Zeta metric"]
        );

        // Search matches description as well as name.
        let data = list_krs(
            &mut model,
            Some(FilterParams {
                search: Some("RETENTION".to_string()),
                ..Default::default()
            }),
        );
        assert_eq!(kr_names(&data), ["Beta metric"]);

        // Pagination.
        let data = list_krs(
            &mut model,
            Some(FilterParams {
                search: Some("metric".to_string()),
                offset: Some(1),
                limit: Some(1),
                ..Default::default()
            }),
        );
        assert_eq!(kr_names(&data), ["Beta metric"]);
        assert_eq!(data["total"], 3);
        assert_eq!(data["offset"], 1);
        assert_eq!(data["limit"], 1);
    }

    #[test]
    fn test_key_result_update_all_fields() {
        let mut model = Model::new();
        let goal = create_goal_in_model(&mut model);
        let kr_id = create_kr(&mut model, &goal, "Signups");
        let lookup = |model: &Model| {
            let uuid = Uuid::parse_str(&kr_id).unwrap();
            model.key_results[&taskflow::domain::KeyResultId(uuid)].clone()
        };

        let resp = process(
            &mut model,
            make_request_with_id_and_data(
                Operation::Update,
                EntityType::KeyResult,
                &kr_id,
                json!({
                    "name": "Paid signups",
                    "description": "Monthly",
                    "target_value": 500.5,
                    "current_value": 120,
                    "unit": "users",
                    "manual_progress": 250
                }),
            ),
        );
        assert!(resp.success, "{:?}", resp.error);
        let data = resp.data.unwrap();
        assert_eq!(data["name"], "Paid signups");
        assert_eq!(data["target_value"], 500.5);
        assert_eq!(data["current_value"], 120.0);
        let kr = lookup(&model);
        assert_eq!(kr.description.as_deref(), Some("Monthly"));
        assert_eq!(kr.unit.as_deref(), Some("users"));
        assert_eq!(kr.manual_progress, Some(100), "progress is clamped");

        // Nulls clear optional fields; non-numeric values are ignored.
        process(
            &mut model,
            make_request_with_id_and_data(
                Operation::Update,
                EntityType::KeyResult,
                &kr_id,
                json!({
                    "description": null,
                    "unit": null,
                    "manual_progress": null,
                    "target_value": "lots"
                }),
            ),
        );
        let kr = lookup(&model);
        assert!(kr.description.is_none());
        assert!(kr.unit.is_none());
        assert!(kr.manual_progress.is_none());
        assert_eq!(kr.target_value.to_string(), "500.5");

        let resp = process(
            &mut model,
            make_request_with_id_and_data(
                Operation::Update,
                EntityType::KeyResult,
                "bad-id",
                json!({"name": "x"}),
            ),
        );
        assert_eq!(resp.error.as_ref().unwrap().code, "INVALID_ID");

        let resp = process(
            &mut model,
            make_request_with_id_and_data(
                Operation::Update,
                EntityType::KeyResult,
                NIL_UUID,
                json!({"name": "x"}),
            ),
        );
        assert_eq!(resp.error.as_ref().unwrap().code, "NOT_FOUND");
    }

    // ========================================================================
    // Habit Update / List Tests
    // ========================================================================

    fn create_habit(model: &mut Model, name: &str) -> String {
        let resp = process(
            model,
            make_request_with_data(Operation::Create, EntityType::Habit, json!({"name": name})),
        );
        assert!(resp.success, "{:?}", resp.error);
        resp.data.unwrap()["id"].as_str().unwrap().to_string()
    }

    fn update_habit_resp(
        model: &mut Model,
        id: &str,
        data: serde_json::Value,
    ) -> PipeResponse<serde_json::Value> {
        process(
            model,
            make_request_with_id_and_data(Operation::Update, EntityType::Habit, id, data),
        )
    }

    fn habit_by_id(model: &Model, id: &str) -> taskflow::domain::Habit {
        let uuid = Uuid::parse_str(id).unwrap();
        model.habits[&taskflow::domain::HabitId(uuid)].clone()
    }

    #[test]
    fn test_habit_update_all_fields() {
        let mut model = Model::new();
        let id = create_habit(&mut model, "Stretch");
        let original_start = habit_by_id(&model, &id).start_date;

        let resp = update_habit_resp(
            &mut model,
            &id,
            json!({
                "frequency": "weekly",
                "archived": true,
                "start_date": "2030-01-01",
                "end_date": "2030-12-31",
                "color": "#00ff00",
                "tags": ["health", 7, "morning"]
            }),
        );
        assert!(resp.success, "{:?}", resp.error);
        let habit = habit_by_id(&model, &id);
        assert!(matches!(habit.frequency, HabitFrequency::Weekly { ref days } if days.len() == 3));
        assert!(habit.archived);
        assert_eq!(habit.start_date.to_string(), "2030-01-01");
        assert_eq!(habit.end_date.unwrap().to_string(), "2030-12-31");
        assert_eq!(habit.color.as_deref(), Some("#00ff00"));
        assert_eq!(habit.tags, ["health", "morning"]);
        assert_ne!(habit.start_date, original_start);

        update_habit_resp(
            &mut model,
            &id,
            json!({
                "frequency": "every_3_days",
                "start_date": "bad",
                "end_date": "bad",
                "description": "x"
            }),
        );
        let habit = habit_by_id(&model, &id);
        assert!(matches!(
            habit.frequency,
            HabitFrequency::EveryNDays { n: 3 }
        ));
        assert_eq!(
            habit.start_date.to_string(),
            "2030-01-01",
            "bad start_date ignored"
        );
        assert!(habit.end_date.is_none(), "bad end_date clears");
        assert_eq!(habit.description.as_deref(), Some("x"));

        update_habit_resp(
            &mut model,
            &id,
            json!({
                "frequency": "fortnightly",
                "end_date": null,
                "color": null,
                "description": null,
                "archived": "yes"
            }),
        );
        let habit = habit_by_id(&model, &id);
        assert!(
            matches!(habit.frequency, HabitFrequency::EveryNDays { n: 3 }),
            "unknown frequency ignored"
        );
        assert!(habit.end_date.is_none());
        assert!(habit.color.is_none());
        assert!(habit.description.is_none());
        assert!(habit.archived, "non-bool archived ignored");
    }

    #[test]
    fn test_habit_update_error_paths() {
        let mut model = Model::new();
        let resp = process(
            &mut model,
            make_request_with_data(Operation::Update, EntityType::Habit, json!({"name": "x"})),
        );
        assert_eq!(resp.error.as_ref().unwrap().code, "MISSING_ID");

        let resp = update_habit_resp(&mut model, "bad", json!({"name": "x"}));
        let err = resp.error.unwrap();
        assert_eq!(err.code, "INVALID_ID");
        assert_eq!(err.message, "Invalid UUID format: bad");
    }

    fn list_habits(model: &mut Model, filters: Option<FilterParams>) -> Vec<String> {
        let req = PipeRequest {
            operation: Operation::List,
            entity: EntityType::Habit,
            id: None,
            data: None,
            filters,
        };
        let resp = process(model, req);
        assert!(resp.success, "{:?}", resp.error);
        resp.data.unwrap()["habits"]
            .as_array()
            .unwrap()
            .iter()
            .map(|h| h["name"].as_str().unwrap().to_string())
            .collect()
    }

    fn list_habits_with_archived(
        model: &mut Model,
        tags: Option<Vec<String>>,
        mode: Option<&str>,
    ) -> Vec<String> {
        list_habits(
            model,
            Some(FilterParams {
                tags,
                tags_mode: mode.map(str::to_string),
                include_completed: Some(true),
                ..Default::default()
            }),
        )
    }

    fn create_tagged_habits(model: &mut Model) {
        let run = create_habit(model, "Run");
        let read = create_habit(model, "Read");
        let old = create_habit(model, "Old habit");
        update_habit_resp(model, &run, json!({"tags": ["Fitness", "morning"]}));
        update_habit_resp(model, &read, json!({"tags": ["mind"]}));
        update_habit_resp(model, &old, json!({"tags": ["fitness"], "archived": true}));
    }

    #[test]
    fn test_habit_list_archived_and_pagination() {
        let mut model = Model::new();
        create_tagged_habits(&mut model);

        // No filters: archived habits included, no sorting applied.
        let mut all = list_habits(&mut model, None);
        all.sort();
        assert_eq!(all, ["Old habit", "Read", "Run"]);

        // Default filters exclude archived and sort by name.
        assert_eq!(
            list_habits(&mut model, Some(FilterParams::default())),
            ["Read", "Run"]
        );
        assert_eq!(
            list_habits_with_archived(&mut model, None, None),
            ["Old habit", "Read", "Run"]
        );

        // Pagination over the sorted list.
        assert_eq!(
            list_habits(
                &mut model,
                Some(FilterParams {
                    include_completed: Some(true),
                    offset: Some(1),
                    limit: Some(1),
                    ..Default::default()
                })
            ),
            ["Read"]
        );
    }

    #[test]
    fn test_habit_list_tag_filters() {
        let mut model = Model::new();
        create_tagged_habits(&mut model);

        assert_eq!(
            list_habits_with_archived(&mut model, strings(&["fitness"]), None),
            ["Old habit", "Run"]
        );
        assert_eq!(
            list_habits_with_archived(&mut model, strings(&["fitness", "MORNING"]), Some("all")),
            ["Run"]
        );
        assert_eq!(
            list_habits_with_archived(&mut model, strings(&["morning", "mind"]), Some("any")),
            ["Read", "Run"]
        );
        assert!(
            list_habits_with_archived(&mut model, strings(&["morning", "mind"]), Some("all"))
                .is_empty()
        );

        // Archived filter applies after the tag filter.
        assert_eq!(
            list_habits(
                &mut model,
                Some(FilterParams {
                    tags: strings(&["fitness"]),
                    ..Default::default()
                })
            ),
            ["Run"]
        );
    }

    // ========================================================================
    // Import Tests
    // ========================================================================

    fn import_resp(
        model: &mut Model,
        data: Option<serde_json::Value>,
    ) -> PipeResponse<serde_json::Value> {
        let req = PipeRequest {
            operation: Operation::Import,
            entity: EntityType::Task,
            id: None,
            data,
            filters: None,
        };
        process(model, req)
    }

    fn export_value(model: &mut Model) -> serde_json::Value {
        let resp = process(model, make_request(Operation::Export, EntityType::Task));
        assert!(resp.success, "{:?}", resp.error);
        resp.data.unwrap()
    }

    /// Populate a model with one of every importable entity and return it.
    fn populated_model() -> Model {
        let mut model = Model::new();
        let task_id = create_task_in_model(&mut model);
        process(
            &mut model,
            make_request_with_data(Operation::Create, EntityType::Project, json!({"name": "P"})),
        );
        process(
            &mut model,
            make_request_with_data(
                Operation::Create,
                EntityType::TimeEntry,
                json!({"task_id": task_id, "duration_minutes": 30}),
            ),
        );
        process(
            &mut model,
            make_request_with_data(
                Operation::Create,
                EntityType::WorkLog,
                json!({"task_id": task_id, "content": "Did things"}),
            ),
        );
        create_habit(&mut model, "H");
        let goal_id = create_goal_in_model(&mut model);
        create_kr(&mut model, &goal_id, "KR");
        let filter = SavedFilter::new("Mine", Filter::default(), SortSpec::default());
        model.saved_filters.insert(filter.id.clone(), filter);

        assert_eq!(model.tasks.len(), 1);
        assert_eq!(model.projects.len(), 1);
        assert_eq!(model.time_entries.len(), 1);
        assert_eq!(model.work_logs.len(), 1);
        assert_eq!(model.habits.len(), 1);
        assert_eq!(model.goals.len(), 1);
        assert_eq!(model.key_results.len(), 1);
        assert_eq!(model.saved_filters.len(), 1);
        model
    }

    #[test]
    fn test_import_missing_data() {
        let mut model = Model::new();
        let resp = import_resp(&mut model, None);
        assert!(!resp.success);
        let err = resp.error.unwrap();
        assert_eq!(err.code, "MISSING_DATA");
        assert_eq!(err.message, "Import data is required");
    }

    #[test]
    fn test_import_invalid_data() {
        let mut model = Model::new();

        // Wrong shape for a field.
        let resp = import_resp(&mut model, Some(json!({"tasks": "nope"})));
        let err = resp.error.unwrap();
        assert_eq!(err.code, "INVALID_DATA");
        assert!(err.message.starts_with("Failed to parse import data:"));

        // Missing required fields.
        let resp = import_resp(&mut model, Some(json!({"version": 1})));
        assert_eq!(resp.error.as_ref().unwrap().code, "INVALID_DATA");

        assert!(model.tasks.is_empty());
    }

    #[test]
    fn test_import_minimal_payload_imports_nothing() {
        let mut model = Model::new();
        let resp = import_resp(
            &mut model,
            Some(json!({
                "tasks": {},
                "projects": [],
                "tags": [],
                "time_entries": [],
                "version": 1
            })),
        );
        assert!(resp.success, "{:?}", resp.error);
        let data = resp.data.unwrap();
        assert_eq!(data["success"], true);
        assert_eq!(
            data["imported"],
            json!({
                "tasks": 0,
                "projects": 0,
                "time_entries": 0,
                "work_logs": 0,
                "habits": 0,
                "goals": 0,
                "key_results": 0,
                "saved_filters": 0
            })
        );
    }

    #[test]
    fn test_import_round_trip_from_export() {
        let mut source = populated_model();
        let exported = export_value(&mut source);

        let mut target = Model::new();
        let resp = import_resp(&mut target, Some(exported.clone()));
        assert!(resp.success, "{:?}", resp.error);
        let data = resp.data.unwrap();
        assert_eq!(
            data["imported"],
            json!({
                "tasks": 1,
                "projects": 1,
                "time_entries": 1,
                "work_logs": 1,
                "habits": 1,
                "goals": 1,
                "key_results": 1,
                "saved_filters": 1
            })
        );

        // Identity is preserved across the round trip.
        let task_id = source.tasks.keys().next().unwrap();
        assert_eq!(target.tasks[task_id].title, source.tasks[task_id].title);
        let goal_id = source.goals.keys().next().unwrap();
        assert!(target.goals.contains_key(goal_id));
        let kr_id = source.key_results.keys().next().unwrap();
        assert_eq!(target.key_results[kr_id].goal_id, *goal_id);
        let filter_id = source.saved_filters.keys().next().unwrap();
        assert_eq!(target.saved_filters[filter_id].name, "Mine");
        assert_eq!(target.projects.len(), 1);
        assert_eq!(target.time_entries.len(), 1);
        assert_eq!(target.work_logs.len(), 1);
        assert_eq!(target.habits.len(), 1);

        // Importing the same payload again is a no-op.
        let resp = import_resp(&mut target, Some(exported));
        let data = resp.data.unwrap();
        for (key, count) in data["imported"].as_object().unwrap() {
            assert_eq!(count, 0, "{key} re-imported");
        }
        assert_eq!(target.tasks.len(), 1);
        assert_eq!(target.saved_filters.len(), 1);
    }

    #[test]
    fn test_import_merges_only_new_entities() {
        let mut source = Model::new();
        let existing = create_task_in_model(&mut source);
        create_task_in_model(&mut source);
        let exported = export_value(&mut source);

        let mut target = Model::new();
        let existing_id = taskflow::domain::TaskId(Uuid::parse_str(&existing).unwrap());
        let mut local_copy = source.tasks[&existing_id].clone();
        local_copy.title = "Local edit".to_string();
        target.tasks.insert(existing_id, local_copy);

        let resp = import_resp(&mut target, Some(exported));
        let data = resp.data.unwrap();
        assert_eq!(data["imported"]["tasks"], 1);
        assert_eq!(target.tasks.len(), 2);
        assert_eq!(
            target.tasks[&existing_id].title, "Local edit",
            "existing entities are not overwritten"
        );
    }

    #[test]
    fn test_import_operation_ignores_entity_type() {
        let mut source = populated_model();
        let exported = export_value(&mut source);
        for entity in [EntityType::Habit, EntityType::Goal, EntityType::SavedFilter] {
            let mut target = Model::new();
            let req = PipeRequest {
                operation: Operation::Import,
                entity,
                id: None,
                data: Some(exported.clone()),
                filters: None,
            };
            let resp = process(&mut target, req);
            assert!(resp.success, "{entity:?}: {:?}", resp.error);
            assert_eq!(target.tasks.len(), 1, "{entity:?}");
        }
    }

    // ========================================================================
    // AuditLog Handler Tests (read-only entity)
    // ========================================================================

    /// Insert an audit entry stamped `minutes` minutes after a fixed epoch and
    /// return its id as a string.
    fn add_audit_entry(
        model: &mut Model,
        task_id: TaskId,
        action: AuditAction,
        minutes: i64,
        changes: Vec<FieldChange>,
    ) -> String {
        let mut entry = AuditLogEntry::new(task_id, action, changes);
        entry.changed_at =
            Utc.with_ymd_and_hms(2030, 1, 1, 0, 0, 0).unwrap() + Duration::minutes(minutes);
        let id = entry.id.0.to_string();
        model.audit_logs.insert(entry.id, entry);
        id
    }

    fn change(field: &str, old: &str, new: &str) -> FieldChange {
        FieldChange {
            field: field.to_string(),
            old_value: old.to_string(),
            new_value: new.to_string(),
        }
    }

    /// Two tasks; task A has created -> modified -> completed, task B has
    /// created -> deleted, interleaved in time. Returns (task_a, task_b, ids in
    /// chronological order).
    fn audit_model() -> (Model, TaskId, TaskId, Vec<String>) {
        let mut model = Model::new();
        let task_a = TaskId(Uuid::new_v4());
        let task_b = TaskId(Uuid::new_v4());
        let ids = vec![
            add_audit_entry(&mut model, task_a, AuditAction::Created, 0, vec![]),
            add_audit_entry(&mut model, task_b, AuditAction::Created, 10, vec![]),
            add_audit_entry(
                &mut model,
                task_a,
                AuditAction::Modified,
                20,
                vec![
                    change("title", "Old", "New"),
                    change("priority", "Low", "High"),
                ],
            ),
            add_audit_entry(&mut model, task_b, AuditAction::Deleted, 30, vec![]),
            add_audit_entry(
                &mut model,
                task_a,
                AuditAction::Completed,
                24 * 60 + 40,
                vec![change("status", "todo", "done")],
            ),
        ];
        (model, task_a, task_b, ids)
    }

    fn audit_filters(filters: FilterParams) -> PipeRequest {
        PipeRequest {
            operation: Operation::List,
            entity: EntityType::AuditLog,
            id: None,
            data: None,
            filters: Some(filters),
        }
    }

    fn listed_ids(resp: &PipeResponse<serde_json::Value>) -> Vec<String> {
        resp.data.as_ref().unwrap()["audit_logs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["id"].as_str().unwrap().to_string())
            .collect()
    }

    #[test]
    fn test_audit_log_list_empty() {
        let mut model = Model::new();
        let req = make_request(Operation::List, EntityType::AuditLog);
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success, "{:?}", resp.error);
        let data = resp.data.unwrap();
        assert_eq!(data["audit_logs"], json!([]));
        assert_eq!(data["total"], 0);
        assert_eq!(data["offset"], 0);
        assert_eq!(data["limit"], 100);
    }

    #[test]
    fn test_audit_log_list_sorted_oldest_first() {
        let (mut model, _, _, ids) = audit_model();
        let req = make_request(Operation::List, EntityType::AuditLog);
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success, "{:?}", resp.error);
        assert_eq!(listed_ids(&resp), ids);
        assert_eq!(resp.metadata.as_ref().unwrap().total, Some(5));

        // Entries keep the export shape: serde-lowercase action, nested changes.
        let entries = &resp.data.as_ref().unwrap()["audit_logs"];
        assert_eq!(entries[0]["action"], "created");
        assert_eq!(entries[0]["changed_at"], "2030-01-01T00:00:00Z");
        assert_eq!(entries[2]["action"], "modified");
        assert_eq!(entries[2]["changes"][0]["field"], "title");
        assert_eq!(entries[2]["changes"][0]["old_value"], "Old");
        assert_eq!(entries[2]["changes"][0]["new_value"], "New");
        assert_eq!(entries[2]["changes"][1]["field"], "priority");
    }

    #[test]
    fn test_audit_log_list_sort_order_desc() {
        let (mut model, _, _, ids) = audit_model();
        let req = audit_filters(FilterParams {
            sort_order: Some("desc".to_string()),
            ..Default::default()
        });
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success, "{:?}", resp.error);
        let mut expected = ids;
        expected.reverse();
        assert_eq!(listed_ids(&resp), expected);
    }

    #[test]
    fn test_audit_log_list_filter_by_task() {
        let (mut model, task_a, task_b, ids) = audit_model();

        let req = audit_filters(FilterParams {
            task_id: Some(task_a.0.to_string()),
            ..Default::default()
        });
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success, "{:?}", resp.error);
        assert_eq!(
            listed_ids(&resp),
            vec![ids[0].clone(), ids[2].clone(), ids[4].clone()]
        );
        assert_eq!(resp.data.unwrap()["total"], 3);

        let req = audit_filters(FilterParams {
            task_id: Some(task_b.0.to_string()),
            ..Default::default()
        });
        let resp = super::super::process_request(&mut model, req);
        assert_eq!(listed_ids(&resp), vec![ids[1].clone(), ids[3].clone()]);

        // `project_id` is not the task filter for this entity.
        let req = audit_filters(FilterParams {
            project_id: Some(task_a.0.to_string()),
            ..Default::default()
        });
        let resp = super::super::process_request(&mut model, req);
        assert_eq!(listed_ids(&resp).len(), ids.len());

        // A task with no history yields an empty list, not an error.
        let req = audit_filters(FilterParams {
            task_id: Some(Uuid::new_v4().to_string()),
            ..Default::default()
        });
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        assert_eq!(resp.data.unwrap()["total"], 0);
    }

    #[test]
    fn test_audit_log_list_filter_by_action() {
        let (mut model, _, _, ids) = audit_model();

        let req = audit_filters(FilterParams {
            status: Some(vec!["created".to_string()]),
            ..Default::default()
        });
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success, "{:?}", resp.error);
        assert_eq!(listed_ids(&resp), vec![ids[0].clone(), ids[1].clone()]);

        // Several actions match any of them; matching ignores case.
        let req = audit_filters(FilterParams {
            status: Some(vec!["Deleted".to_string(), "COMPLETED".to_string()]),
            ..Default::default()
        });
        let resp = super::super::process_request(&mut model, req);
        assert_eq!(listed_ids(&resp), vec![ids[3].clone(), ids[4].clone()]);

        // An unknown action matches nothing.
        let req = audit_filters(FilterParams {
            status: Some(vec!["exploded".to_string()]),
            ..Default::default()
        });
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        assert_eq!(resp.data.unwrap()["total"], 0);
    }

    #[test]
    fn test_audit_log_list_filter_by_date() {
        let (mut model, _, _, ids) = audit_model();

        // Only the completion happened on 2030-01-02.
        let req = audit_filters(FilterParams {
            due_after: Some("2030-01-02".to_string()),
            ..Default::default()
        });
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success, "{:?}", resp.error);
        assert_eq!(listed_ids(&resp), vec![ids[4].clone()]);

        let req = audit_filters(FilterParams {
            due_before: Some("2030-01-01".to_string()),
            ..Default::default()
        });
        let resp = super::super::process_request(&mut model, req);
        assert_eq!(listed_ids(&resp), ids[..4].to_vec());

        // Bounds are inclusive on both ends.
        let req = audit_filters(FilterParams {
            due_after: Some("2030-01-01".to_string()),
            due_before: Some("2030-01-02".to_string()),
            ..Default::default()
        });
        let resp = super::super::process_request(&mut model, req);
        assert_eq!(listed_ids(&resp).len(), 5);

        // An unparsable date is ignored rather than rejected.
        let req = audit_filters(FilterParams {
            due_after: Some("yesterday".to_string()),
            ..Default::default()
        });
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success);
        assert_eq!(listed_ids(&resp).len(), 5);
    }

    #[test]
    fn test_audit_log_list_limit_returns_newest() {
        let (mut model, _, _, ids) = audit_model();

        let req = audit_filters(FilterParams {
            limit: Some(2),
            ..Default::default()
        });
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success, "{:?}", resp.error);
        // The two most recent entries, still oldest first within the page.
        assert_eq!(listed_ids(&resp), vec![ids[3].clone(), ids[4].clone()]);
        let data = resp.data.unwrap();
        assert_eq!(data["total"], 5, "total counts before paging");
        assert_eq!(data["limit"], 2);

        // Offset walks further back from the newest entry.
        let req = audit_filters(FilterParams {
            limit: Some(2),
            offset: Some(2),
            ..Default::default()
        });
        let resp = super::super::process_request(&mut model, req);
        assert_eq!(listed_ids(&resp), vec![ids[1].clone(), ids[2].clone()]);
        assert_eq!(resp.data.unwrap()["offset"], 2);

        // With sort_order desc the same page comes back newest first.
        let req = audit_filters(FilterParams {
            limit: Some(2),
            sort_order: Some("desc".to_string()),
            ..Default::default()
        });
        let resp = super::super::process_request(&mut model, req);
        assert_eq!(listed_ids(&resp), vec![ids[4].clone(), ids[3].clone()]);
    }

    #[test]
    fn test_audit_log_list_same_timestamp_is_deterministic() {
        let mut model = Model::new();
        let task = TaskId(Uuid::new_v4());
        let mut ids: Vec<String> = (0..4)
            .map(|_| add_audit_entry(&mut model, task, AuditAction::Modified, 0, vec![]))
            .collect();
        ids.sort();

        let req = make_request(Operation::List, EntityType::AuditLog);
        let first = listed_ids(&super::super::process_request(&mut model, req));
        assert_eq!(first, ids, "ties break on id ascending");

        for _ in 0..3 {
            let req = make_request(Operation::List, EntityType::AuditLog);
            let again = listed_ids(&super::super::process_request(&mut model, req));
            assert_eq!(again, first);
        }
    }

    #[test]
    fn test_audit_log_get() {
        let (mut model, task_a, _, ids) = audit_model();
        let req = make_request_with_id(Operation::Get, EntityType::AuditLog, &ids[2]);
        let resp = super::super::process_request(&mut model, req);
        assert!(resp.success, "{:?}", resp.error);
        let data = resp.data.unwrap();
        assert_eq!(data["id"], ids[2]);
        assert_eq!(data["task_id"], task_a.0.to_string());
        assert_eq!(data["action"], "modified");
        assert_eq!(data["changed_at"], "2030-01-01T00:20:00Z");
        assert_eq!(data["changes"].as_array().unwrap().len(), 2);
        assert!(resp.metadata.is_none(), "get carries no list metadata");
    }

    #[test]
    fn test_audit_log_get_not_found() {
        let (mut model, _, _, _) = audit_model();
        let missing = Uuid::new_v4().to_string();
        let req = make_request_with_id(Operation::Get, EntityType::AuditLog, &missing);
        let resp = super::super::process_request(&mut model, req);
        assert!(!resp.success);
        let err = resp.error.unwrap();
        assert_eq!(err.code, "NOT_FOUND");
        assert!(err.message.contains(&missing), "{}", err.message);
    }

    #[test]
    fn test_audit_log_get_invalid_and_missing_id() {
        let mut model = Model::new();

        let req = make_request_with_id(Operation::Get, EntityType::AuditLog, "not-a-uuid");
        let resp = super::super::process_request(&mut model, req);
        assert_eq!(resp.error.unwrap().code, "INVALID_ID");

        let req = make_request(Operation::Get, EntityType::AuditLog);
        let resp = super::super::process_request(&mut model, req);
        assert_eq!(resp.error.unwrap().code, "MISSING_ID");
    }

    #[test]
    fn test_audit_log_write_operations_are_read_only() {
        let (mut model, task_a, _, ids) = audit_model();
        let before = model.audit_logs.len();

        let requests = vec![
            make_request_with_data(
                Operation::Create,
                EntityType::AuditLog,
                json!({"task_id": task_a.0.to_string(), "action": "created"}),
            ),
            make_request_with_id_and_data(
                Operation::Update,
                EntityType::AuditLog,
                &ids[0],
                json!({"action": "deleted"}),
            ),
            make_request_with_id(Operation::Delete, EntityType::AuditLog, &ids[0]),
        ];
        for req in requests {
            let op = req.operation;
            let resp = super::super::process_request(&mut model, req);
            assert!(!resp.success, "{op:?} must fail");
            let err = resp.error.unwrap();
            assert_eq!(err.code, "READ_ONLY", "{op:?}");
            assert_eq!(
                err.message,
                "audit_log is append-only and written by the application"
            );
        }

        assert_eq!(model.audit_logs.len(), before, "nothing was written");
        assert_eq!(
            model.audit_logs[&AuditLogEntryId(Uuid::parse_str(&ids[0]).unwrap())].action,
            AuditAction::Created
        );

        // Export/import still answer like every other entity handler.
        let req = make_request(Operation::Import, EntityType::AuditLog);
        let err = handlers::audit_log::handle(&mut model, &req).unwrap_err();
        assert_eq!(err.code, "INVALID_OPERATION");
    }

    #[test]
    fn test_audit_log_csv_output() {
        let (mut model, task_a, task_b, ids) = audit_model();
        let req = make_request(Operation::List, EntityType::AuditLog);
        let resp = super::super::process_request(&mut model, req);
        let out = format::serialize_response(&resp, OutputFormat::Csv).unwrap();

        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 6, "header plus one row per entry: {out}");
        assert_eq!(lines[0], "id,task_id,changed_at,action,changes");
        assert_eq!(
            lines[1],
            format!("{},{},2030-01-01T00:00:00Z,created,", ids[0], task_a.0)
        );
        assert_eq!(
            lines[2],
            format!("{},{},2030-01-01T00:10:00Z,created,", ids[1], task_b.0)
        );
        // Field changes are flattened to `field: old -> new; ...`.
        assert_eq!(
            lines[3],
            format!(
                "{},{},2030-01-01T00:20:00Z,modified,title: Old -> New; priority: Low -> High",
                ids[2], task_a.0
            )
        );
        assert_eq!(
            lines[4],
            format!("{},{},2030-01-01T00:30:00Z,deleted,", ids[3], task_b.0)
        );
        assert_eq!(
            lines[5],
            format!(
                "{},{},2030-01-02T00:40:00Z,completed,status: todo -> done",
                ids[4], task_a.0
            )
        );
    }

    #[test]
    fn test_audit_log_csv_quotes_changes_with_commas() {
        let mut model = Model::new();
        let task = TaskId(Uuid::new_v4());
        add_audit_entry(
            &mut model,
            task,
            AuditAction::Modified,
            0,
            vec![change("tags", "a, b", "a, b, c")],
        );
        let req = make_request(Operation::List, EntityType::AuditLog);
        let resp = super::super::process_request(&mut model, req);
        let out = format::serialize_response(&resp, OutputFormat::Csv).unwrap();
        assert!(
            out.lines()
                .nth(1)
                .unwrap()
                .ends_with(",modified,\"tags: a, b -> a, b, c\""),
            "{out}"
        );
    }

    #[test]
    fn test_audit_log_run_pipe_loop_list() {
        let (mut model, task_a, _, ids) = audit_model();
        let out = run_loop(
            &mut model,
            concat!(
                r#"{"operation":"list","entity":"audit_log"}"#,
                "\n",
                r#"{"operation":"list","entity":"audit_log","filters":{"status":["completed"],"limit":1}}"#,
                "\n",
                r#"{"operation":"create","entity":"audit_log","data":{}}"#,
                "\n"
            ),
            OutputFormat::Json,
        );
        let responses = json_stream(&out);
        assert_eq!(responses.len(), 3, "{out}");

        assert_eq!(responses[0]["success"], true);
        assert_eq!(responses[0]["data"]["total"], 5);
        assert_eq!(responses[0]["metadata"]["total"], 5);
        assert_eq!(
            responses[0]["data"]["audit_logs"].as_array().unwrap().len(),
            5
        );
        assert_eq!(responses[0]["data"]["audit_logs"][0]["id"], ids[0]);

        assert_eq!(responses[1]["success"], true);
        assert_eq!(responses[1]["data"]["total"], 1);
        assert_eq!(responses[1]["data"]["audit_logs"][0]["id"], ids[4]);
        assert_eq!(
            responses[1]["data"]["audit_logs"][0]["task_id"],
            task_a.0.to_string()
        );

        assert_eq!(responses[2]["success"], false);
        assert_eq!(responses[2]["error"]["code"], "READ_ONLY");
    }

    // ========================================================================
    // Run Loop Tests
    // ========================================================================

    /// A reader whose first read fails and which then reports EOF.
    struct FailOnceReader {
        failed: bool,
    }

    impl std::io::Read for FailOnceReader {
        fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
            if self.failed {
                Ok(0)
            } else {
                self.failed = true;
                Err(std::io::Error::other("boom"))
            }
        }
    }

    impl std::io::BufRead for FailOnceReader {
        fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
            if self.failed {
                Ok(&[])
            } else {
                self.failed = true;
                Err(std::io::Error::other("boom"))
            }
        }

        fn consume(&mut self, _amt: usize) {}
    }

    /// A writer that rejects every write.
    struct FailingWriter;

    impl std::io::Write for FailingWriter {
        fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("disk full"))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn run_loop(model: &mut Model, input: &str, format: OutputFormat) -> String {
        let mut out = Vec::new();
        super::super::run_pipe_loop(model, input.as_bytes(), &mut out, format).unwrap();
        String::from_utf8(out).unwrap()
    }

    fn json_stream(output: &str) -> Vec<serde_json::Value> {
        serde_json::Deserializer::from_str(output)
            .into_iter::<serde_json::Value>()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }

    #[test]
    fn test_parse_output_format() {
        let parse = super::super::parse_output_format;
        assert_eq!(parse("json"), OutputFormat::Json);
        assert_eq!(parse("JSON"), OutputFormat::Json);
        assert_eq!(parse("yaml"), OutputFormat::Yaml);
        assert_eq!(parse("yml"), OutputFormat::Yaml);
        assert_eq!(parse("Csv"), OutputFormat::Csv);
        assert_eq!(
            parse("xml"),
            OutputFormat::Json,
            "unknown falls back to JSON"
        );
        assert_eq!(parse(""), OutputFormat::Json);
    }

    #[test]
    fn test_run_pipe_loop_processes_each_line() {
        let mut model = Model::new();
        let input = concat!(
            r#"{"operation":"create","entity":"task","data":{"title":"From pipe"}}"#,
            "\n\n   \n",
            r#"  {"operation":"list","entity":"task"}  "#,
            "\n",
            "this is not json\n",
            r#"{"operation":"get","entity":"task","id":"nope"}"#,
            "\n",
        );
        let out = run_loop(&mut model, input, OutputFormat::Json);
        let responses = json_stream(&out);
        assert_eq!(responses.len(), 4, "blank lines produce no output: {out}");

        assert_eq!(responses[0]["success"], true);
        assert_eq!(responses[0]["data"]["title"], "From pipe");

        assert_eq!(responses[1]["success"], true);
        assert_eq!(responses[1]["data"]["total"], 1);
        assert_eq!(responses[1]["metadata"]["total"], 1);

        assert_eq!(responses[2]["success"], false);
        assert_eq!(responses[2]["error"]["code"], "PARSE_ERROR");
        assert!(responses[2]["error"]["message"]
            .as_str()
            .unwrap()
            .starts_with("Failed to parse request:"));

        assert_eq!(responses[3]["error"]["code"], "INVALID_ID");

        assert_eq!(model.tasks.len(), 1);
        assert!(out.ends_with('\n'));
    }

    #[test]
    fn test_run_pipe_loop_empty_input_writes_nothing() {
        let mut model = Model::new();
        assert_eq!(run_loop(&mut model, "", OutputFormat::Json), "");
        assert_eq!(run_loop(&mut model, "\n\n", OutputFormat::Yaml), "");
    }

    #[test]
    fn test_run_pipe_loop_yaml_and_csv_formats() {
        let mut model = create_test_model();

        let out = run_loop(
            &mut model,
            r#"{"operation":"list","entity":"task"}"#,
            OutputFormat::Yaml,
        );
        assert!(out.contains("success: true"), "{out}");
        assert!(out.contains("total: 3"), "{out}");

        let out = run_loop(
            &mut model,
            concat!(
                r#"{"operation":"list","entity":"task"}"#,
                "\n",
                r#"{"operation":"get","entity":"task","id":"bad"}"#,
                "\n"
            ),
            OutputFormat::Csv,
        );
        let mut lines = out.lines();
        assert_eq!(
            lines.next().unwrap(),
            "id,title,status,priority,project_id,due_date,scheduled_date,tags,estimated_minutes,created_at,completed_at"
        );
        let csv_rows = lines
            .by_ref()
            .take_while(|l| !l.starts_with('{'))
            .filter(|l| !l.is_empty())
            .count();
        assert_eq!(csv_rows, 3, "one CSV row per task: {out}");
        // The error response falls back to JSON after the CSV block.
        assert!(out.contains("\"code\": \"INVALID_ID\""), "{out}");
    }

    #[test]
    fn test_run_pipe_loop_reports_read_errors() {
        let mut model = Model::new();
        let mut out = Vec::new();
        super::super::run_pipe_loop(
            &mut model,
            FailOnceReader { failed: false },
            &mut out,
            OutputFormat::Json,
        )
        .unwrap();
        let responses = json_stream(std::str::from_utf8(&out).unwrap());
        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0]["success"], false);
        assert_eq!(responses[0]["error"]["code"], "IO_ERROR");
        assert_eq!(
            responses[0]["error"]["message"],
            "Failed to read stdin: boom"
        );
    }

    #[test]
    fn test_run_pipe_loop_propagates_write_errors() {
        let mut model = Model::new();
        let input = r#"{"operation":"list","entity":"task"}"#;
        let err = super::super::run_pipe_loop(
            &mut model,
            input.as_bytes(),
            &mut FailingWriter,
            OutputFormat::Json,
        )
        .unwrap_err();
        assert!(err.to_string().contains("disk full"), "{err}");
    }

    #[test]
    fn test_write_response_formats_and_terminates_line() {
        let response = super::super::create_error_response("X", "why".to_string());
        let mut out = Vec::new();
        super::super::write_response(&mut out, &response, OutputFormat::Yaml).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.ends_with('\n'));
        assert!(text.contains("code: X"));
        assert!(text.contains("message: why"));

        let err = super::super::write_response(&mut FailingWriter, &response, OutputFormat::Json)
            .unwrap_err();
        assert!(err.to_string().contains("disk full"));
    }
}
