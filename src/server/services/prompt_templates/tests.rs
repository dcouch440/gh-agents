#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use crate::db::fixtures::fixtures::*;
    use crate::db::traits::MockPromptTemplateRepo;
    use crate::db::PromptTemplateRow;
    use crate::server::services::prompt_templates::*;
    use crate::server::services::ServiceError;

    fn make_template(user_id: Option<Uuid>, name: &str) -> PromptTemplateRow {
        PromptTemplateRow {
            user_id,
            name: name.to_string(),
            ..prompt_template(user_id.unwrap_or_else(Uuid::new_v4))
        }
    }

    #[tokio::test]
    async fn create_rejects_empty_name() {
        let repo = MockPromptTemplateRepo::new();
        let user_id = Uuid::new_v4();

        let result =
            create_prompt_template(&repo, user_id, "  ".to_string(), "content".to_string()).await;
        assert!(matches!(result, Err(ServiceError::Validation(_))));
    }

    #[tokio::test]
    async fn create_succeeds_with_valid_input() {
        let user_id = Uuid::new_v4();

        let mut repo = MockPromptTemplateRepo::new();
        repo.expect_create_prompt_template()
            .returning(move |uid, name, content| {
                Ok(PromptTemplateRow {
                    id: Uuid::new_v4(),
                    user_id: uid,
                    name,
                    content,
                    created_at: Utc::now(),
                    version: 1,
                })
            });

        let result = create_prompt_template(
            &repo,
            user_id,
            "My Template".to_string(),
            "content".to_string(),
        )
        .await;
        let row = result.unwrap();
        assert_eq!(row.name, "My Template");
        assert_eq!(row.user_id, Some(user_id));
    }

    #[tokio::test]
    async fn get_system_template_visible_to_any_user() {
        let system_template = make_template(None, "System Template");
        let template_id = system_template.id;
        let template_clone = system_template.clone();

        let mut repo = MockPromptTemplateRepo::new();
        repo.expect_get_prompt_template()
            .returning(move |_| Ok(Some(template_clone.clone())));

        let random_user = Uuid::new_v4();
        let result = get_prompt_template(&repo, random_user, template_id).await;
        let row = result.unwrap();
        assert_eq!(row.name, "System Template");
        assert!(row.user_id.is_none());
    }

    #[tokio::test]
    async fn get_user_template_rejects_non_owner() {
        let owner_id = Uuid::new_v4();
        let attacker_id = Uuid::new_v4();
        let template = make_template(Some(owner_id), "Private Template");
        let template_id = template.id;
        let template_clone = template.clone();

        let mut repo = MockPromptTemplateRepo::new();
        repo.expect_get_prompt_template()
            .returning(move |_| Ok(Some(template_clone.clone())));

        let result = get_prompt_template(&repo, attacker_id, template_id).await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }

    #[tokio::test]
    async fn update_rejects_system_template() {
        let system_template = make_template(None, "System Template");
        let template_id = system_template.id;
        let template_clone = system_template.clone();

        let mut repo = MockPromptTemplateRepo::new();
        repo.expect_get_prompt_template()
            .returning(move |_| Ok(Some(template_clone.clone())));

        let user_id = Uuid::new_v4();
        let result = update_prompt_template(
            &repo,
            user_id,
            template_id,
            Some("New Name".to_string()),
            None,
        )
        .await;
        assert!(matches!(result, Err(ServiceError::NotFound(_))));
    }
}
