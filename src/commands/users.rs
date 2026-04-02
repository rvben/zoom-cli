use crate::api::{ApiError, ZoomClient};
use crate::output::{self, OutputConfig};

pub async fn list(
    client: &mut ZoomClient,
    out: &OutputConfig,
    status: Option<&str>,
) -> Result<(), ApiError> {
    let result = client.list_users(status).await?;

    if out.json {
        out.print_data(
            &serde_json::to_string_pretty(&serde_json::json!({
                "users": result.users,
                "totalRecords": result.total_records
            }))
            .expect("serialize"),
        );
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
        out.print_data(&output::table(
            &["ID", "EMAIL", "NAME", "STATUS"],
            &rows,
        ));
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
        out.print_data(
            &serde_json::to_string_pretty(&user).expect("serialize"),
        );
    } else {
        let name = user.display_name.clone()
            .or_else(|| {
                match (&user.first_name, &user.last_name) {
                    (Some(f), Some(l)) => Some(format!("{f} {l}")),
                    (Some(f), None) => Some(f.clone()),
                    _ => None,
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ZoomClient;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_out_json() -> OutputConfig {
        OutputConfig { json: true, quiet: true }
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

        let mut client = ZoomClient::new_for_test(
            format!("{}/v2", server.uri()), server.uri(), "tok".into()
        );
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

        let mut client = ZoomClient::new_for_test(
            format!("{}/v2", server.uri()), server.uri(), "tok".into()
        );
        let err = get(&mut client, &test_out_json(), "nobody@example.com").await.unwrap_err();
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

        let mut client = ZoomClient::new_for_test(
            format!("{}/v2", server.uri()), server.uri(), "tok".into()
        );
        me(&mut client, &test_out_json()).await.unwrap();
    }
}
