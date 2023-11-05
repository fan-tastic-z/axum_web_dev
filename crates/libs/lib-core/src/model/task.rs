use crate::ctx::Ctx;
use crate::model::base::{self, DbBmc};
use crate::model::ModelManager;
use crate::model::Result;
use modql::field::Fields;
use modql::filter::{FilterNodes, ListOptions, OpValsBool, OpValsString};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// region:    --- Task Types

#[derive(Debug, Clone, Fields, FromRow, Serialize)]
pub struct Task {
	pub id: i64,
	pub title: String,
	pub done: bool,
}

#[derive(Deserialize, Fields)]
pub struct TaskForCreate {
	pub title: String,
}

#[derive(Deserialize, Fields, Default)]
pub struct TaskForUpdate {
	pub title: Option<String>,
	pub done: Option<bool>,
}

#[derive(FilterNodes, Deserialize, Default, Debug)]
pub struct TaskFilter {
	title: Option<OpValsString>,
	done: Option<OpValsBool>,
}

// endregion: --- Task Types

// region:    --- TaskBmc
pub struct TaskBmc;

impl DbBmc for TaskBmc {
	const TABLE: &'static str = "task";
}

impl TaskBmc {
	pub async fn create(
		ctx: &Ctx,
		mm: &ModelManager,
		task_c: TaskForCreate,
	) -> Result<i64> {
		base::create::<Self, _>(ctx, mm, task_c).await
	}

	pub async fn get(ctx: &Ctx, mm: &ModelManager, id: i64) -> Result<Task> {
		base::get::<Self, _>(ctx, mm, id).await
	}

	pub async fn list(
		ctx: &Ctx,
		mm: &ModelManager,
		filter: Option<TaskFilter>,
		list_options: Option<ListOptions>,
	) -> Result<Vec<Task>> {
		base::list::<Self, _, _>(ctx, mm, filter, list_options).await
	}

	pub async fn update(
		ctx: &Ctx,
		mm: &ModelManager,
		id: i64,
		task_u: TaskForUpdate,
	) -> Result<()> {
		base::update::<Self, _>(ctx, mm, id, task_u).await
	}

	pub async fn delete(ctx: &Ctx, mm: &ModelManager, id: i64) -> Result<()> {
		base::delete::<Self>(ctx, mm, id).await
	}
}
// endregion: --- TaskBmc

// region:    --- TestBmc
#[cfg(test)]
mod tests {
	#![allow(unused)]
	use crate::{_dev_utils, model::Error};

	use super::*;
	use anyhow::Result;
	use modql::filter::OpValString;
	use serial_test::serial;

	#[serial]
	#[tokio::test]
	async fn test_create_ok() -> Result<()> {
		// -- Setup & Fixtures
		let mm = _dev_utils::init_test().await;
		let ctx = Ctx::root_ctx();
		let fx_title = "test_create_ok title";

		// -- Exec
		let task_c = TaskForCreate {
			title: fx_title.to_string(),
		};
		let id = TaskBmc::create(&ctx, &mm, task_c).await?;

		// -- Check
		let task = TaskBmc::get(&ctx, &mm, id).await?;
		assert_eq!(task.title, fx_title);

		// -- Clean
		TaskBmc::delete(&ctx, &mm, id).await?;

		Ok(())
	}

	#[serial]
	#[tokio::test]
	async fn test_get_err_not_found() -> Result<()> {
		// -- Setup & Fixtures
		let mm = _dev_utils::init_test().await;
		let ctx = Ctx::root_ctx();
		let fx_id = 100;

		// -- Exec
		let res = TaskBmc::get(&ctx, &mm, fx_id).await;

		// -- Check
		assert!(
			matches!(
				res,
				Err(Error::EntityNotFound {
					entity: "task",
					id: 100
				})
			),
			"EntityNotFound not matching"
		);

		Ok(())
	}

	#[serial]
	#[tokio::test]
	async fn test_list_all_ok() -> Result<()> {
		// -- Setup & Fixtures

		let mm = _dev_utils::init_test().await;
		let ctx = Ctx::root_ctx();
		let fx_titles = &["test_list_all_ok-task 01", "test_list_all_ok-task 02"];
		_dev_utils::seed_tasks(&ctx, &mm, fx_titles).await?;

		// -- Exec
		let tasks = TaskBmc::list(&ctx, &mm, None, None).await?;

		// -- Check
		let tasks: Vec<Task> = tasks
			.into_iter()
			.filter(|t| t.title.starts_with("test_list_all_ok-task"))
			.collect();
		assert_eq!(tasks.len(), 2, "number of seeded tasks.");

		// -- Clean
		for task in tasks.iter() {
			TaskBmc::delete(&ctx, &mm, task.id).await?;
		}

		Ok(())
	}

	#[serial]
	#[tokio::test]
	async fn test_list_by_title_contains_ok() -> Result<()> {
		// -- Setup & Fixtures
		let mm = _dev_utils::init_test().await;
		let ctx = Ctx::root_ctx();
		let fx_titles = &[
			"test_list_by_title_contains_ok 01",
			"test_list_by_title_contains_ok 02.1",
			"test_list_by_title_contains_ok 02.2",
		];

		_dev_utils::seed_tasks(&ctx, &mm, fx_titles).await?;

		// -- Exec
		let filter = TaskFilter {
			title: Some(
				OpValString::Contains("by_title_contains_ok 02".to_string()).into(),
			),
			..Default::default()
		};
		let tasks = TaskBmc::list(&ctx, &mm, Some(filter), None).await?;

		// -- Check
		assert_eq!(tasks.len(), 2);

		// -- Cleanup
		// Will delete associate tasks
		for task in tasks {
			TaskBmc::delete(&ctx, &mm, task.id).await?;
		}

		Ok(())
	}

	#[serial]
	#[tokio::test]
	async fn test_update_ok() -> Result<()> {
		// -- Setup & Fixtures
		let mm = _dev_utils::init_test().await;
		let ctx = Ctx::root_ctx();
		let fx_title = "test_update_ok - task 01";
		let fx_title_new = "test_update_ok - task 01 - new";
		let fx_task = _dev_utils::seed_tasks(&ctx, &mm, &[fx_title])
			.await?
			.remove(0);

		// Exec
		TaskBmc::update(
			&ctx,
			&mm,
			fx_task.id,
			TaskForUpdate {
				title: Some(fx_title_new.to_string()),
				..Default::default()
			},
		)
		.await?;

		// -- Check
		let task = TaskBmc::get(&ctx, &mm, fx_task.id).await?;
		assert_eq!(task.title, fx_title_new);

		Ok(())
	}

	#[serial]
	#[tokio::test]
	async fn test_delete_err_not_found() -> Result<()> {
		// -- Setup & Fixtures
		let mm = _dev_utils::init_test().await;
		let ctx = Ctx::root_ctx();
		let fx_id = 100;

		// -- Exec
		let res = TaskBmc::delete(&ctx, &mm, fx_id).await;

		// -- Check
		assert!(
			matches!(
				res,
				Err(Error::EntityNotFound {
					entity: "task",
					id: 100
				})
			),
			"EntityNotFound not matching"
		);

		Ok(())
	}
}

// endregion: --- TestBmc