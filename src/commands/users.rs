use crate::api::{ApiError, ZoomClient};
use crate::output::{self, OutputConfig};

pub async fn list(
    client: &mut ZoomClient,
    out: &OutputConfig,
    status: Option<&str>,
) -> Result<(), ApiError> {
    let result = client.list_users(status).await?;

    if out.json {
        out.print_data(&serde_json::to_string_pretty(&result).expect("serialize"));
    } else {
        if result.users.is_empty() {
            out.print_message("No users found.");
            return Ok(());
        }
        let rows: Vec<Vec<String>> = result
            .users
            .iter()
            .map(|u| {
                vec![
                    u.id.clone(),
                    u.email.clone(),
                    u.display_name.clone().unwrap_or_default(),
                    u.status.clone().unwrap_or_default(),
                ]
            })
            .collect();
        out.print_data(&output::table(&["ID", "EMAIL", "NAME", "STATUS"], &rows));
        if let Some(total) = result.total_records {
            out.print_message(&format!("{total} user(s) total"));
        }
    }
    Ok(())
}

pub async fn get(
    client: &mut ZoomClient,
    out: &OutputConfig,
    id_or_email: &str,
) -> Result<(), ApiError> {
    let user = client.get_user(id_or_email).await?;

    if out.json {
        out.print_data(&serde_json::to_string_pretty(&user).expect("serialize"));
    } else {
        let name = user
            .display_name
            .clone()
            .or_else(|| match (&user.first_name, &user.last_name) {
                (Some(f), Some(l)) => Some(format!("{f} {l}")),
                (Some(f), None) => Some(f.clone()),
                _ => None,
            })
            .unwrap_or_default();
        out.print_data(&output::kv_block(&[
            ("id", user.id.clone()),
            ("email", user.email.clone()),
            ("name", name),
            ("status", user.status.clone().unwrap_or_default()),
        ]));
    }
    Ok(())
}

pub async fn me(client: &mut ZoomClient, out: &OutputConfig) -> Result<(), ApiError> {
    get(client, out, "me").await
}

pub async fn create(
    client: &mut ZoomClient,
    out: &OutputConfig,
    email: String,
    first_name: Option<String>,
    last_name: Option<String>,
    user_type: u8,
) -> Result<(), ApiError> {
    use crate::api::types::{CreateUserInfo, CreateUserRequest};
    let req = CreateUserRequest {
        action: "create".into(),
        user_info: CreateUserInfo {
            email,
            user_type,
            first_name,
            last_name,
        },
    };
    let user = client.create_user(req).await?;
    if out.json {
        out.print_data(&serde_json::to_string_pretty(&user).expect("serialize"));
    } else {
        out.print_message(&format!("Created user: {} ({})", user.email, user.id));
    }
    Ok(())
}

pub async fn deactivate(
    client: &mut ZoomClient,
    out: &OutputConfig,
    id_or_email: &str,
) -> Result<(), ApiError> {
    client.set_user_status(id_or_email, "deactivate").await?;
    out.print_message(&format!("User {} deactivated.", id_or_email));
    Ok(())
}

pub async fn activate(
    client: &mut ZoomClient,
    out: &OutputConfig,
    id_or_email: &str,
) -> Result<(), ApiError> {
    client.set_user_status(id_or_email, "activate").await?;
    out.print_message(&format!("User {} activated.", id_or_email));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ZoomClient;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_out_json() -> OutputConfig {
        OutputConfig {
            json: true,
            quiet: true,
        }
    }

    #[tokio::test]
    async fn users_list_json_includes_total_records() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v2/users"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "users": [
                    {"id": "u1", "email": "a@test.com", "display_name": "Alice", "status": "active"}
                ],
                "total_records": 1
            })))
            .mount(&server)
            .await;

        let mut client =
            ZoomClient::new_for_test(format!("{}/v2", server.uri()), server.uri(), "tok".into());
        list(&mut client, &test_out_json(), None).await.unwrap();
    }

    #[tokio::test]
    async fn users_get_returns_not_found_for_unknown_user() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v2/users/nobody@example.com"))
            .respond_with(ResponseTemplate::new(404).set_body_string("User not found"))
            .mount(&server)
            .await;

        let mut client =
            ZoomClient::new_for_test(format!("{}/v2", server.uri()), server.uri(), "tok".into());
        let err = get(&mut client, &test_out_json(), "nobody@example.com")
            .await
            .unwrap_err();
        assert!(matches!(err, ApiError::NotFound(_)));
    }

    #[tokio::test]
    async fn users_me_delegates_to_get_with_me() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v2/users/me"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "self-user", "email": "me@example.com", "display_name": "Me"
            })))
            .mount(&server)
            .await;

        let mut client =
            ZoomClient::new_for_test(format!("{}/v2", server.uri()), server.uri(), "tok".into());
        me(&mut client, &test_out_json()).await.unwrap();
    }

    #[tokio::test]
    async fn users_create_returns_created_user() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/users"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": "new-user-id",
                "email": "jane@example.com",
                "first_name": "Jane",
                "last_name": "Doe",
                "status": "active"
            })))
            .mount(&server)
            .await;

        let mut client =
            ZoomClient::new_for_test(format!("{}/v2", server.uri()), server.uri(), "tok".into());
        create(
            &mut client,
            &test_out_json(),
            "jane@example.com".into(),
            Some("Jane".into()),
            Some("Doe".into()),
            1,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn users_deactivate_sends_correct_action() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/v2/users/u1/status"))
            .and(wiremock::matchers::body_json(
                serde_json::json!({"action": "deactivate"}),
            ))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let mut client =
            ZoomClient::new_for_test(format!("{}/v2", server.uri()), server.uri(), "tok".into());
        deactivate(&mut client, &test_out_json(), "u1")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn users_activate_sends_correct_action() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/v2/users/u1/status"))
            .and(wiremock::matchers::body_json(
                serde_json::json!({"action": "activate"}),
            ))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let mut client =
            ZoomClient::new_for_test(format!("{}/v2", server.uri()), server.uri(), "tok".into());
        activate(&mut client, &test_out_json(), "u1").await.unwrap();
    }
}
