use lib_base::time::now_utc;
use modql::field::{Field, Fields, HasFields};
use modql::filter::{IntoSeaError, ListOptions};
use modql::SIden;
use sea_query::{
	Condition, Expr, Iden, IntoIden, PostgresQueryBuilder, Query, TableRef,
};
use sea_query_binder::SqlxBinder;
use sqlx::postgres::PgRow;
use sqlx::FromRow;

use crate::ctx::Ctx;
use crate::model::ModelManager;
use crate::model::{Error, Result};

#[derive(Iden)]
pub enum CommonIden {
	Id,
}

#[derive(Iden)]
pub enum TimestampIden {
	Cid,
	Ctime,
	Mid,
	Mtime,
}

pub trait DbBmc {
	const TABLE: &'static str;

	fn table_ref() -> TableRef {
		TableRef::Table(SIden(Self::TABLE).into_iden())
	}
}

pub async fn create<MC, E>(ctx: &Ctx, mm: &ModelManager, data: E) -> Result<i64>
where
	MC: DbBmc,
	E: HasFields,
{
	let db = mm.db();

	// -- Extract fields (name / sea-query value expression)
	let mut fields = data.not_none_fields();
	add_timestamps_for_create(&mut fields, ctx.user_id());
	let (columns, sea_values) = fields.for_sea_insert();

	// -- Build query
	let mut query = Query::insert();
	query
		.into_table(MC::table_ref())
		.columns(columns)
		.values(sea_values)?
		.returning(Query::returning().columns([CommonIden::Id]));
	// -- Exec query
	let (sql, values) = query.build_sqlx(PostgresQueryBuilder);
	let (id,) = sqlx::query_as_with::<_, (i64,), _>(&sql, values)
		.fetch_one(db)
		.await?;

	Ok(id)
}

pub async fn get<MC, E>(_ctx: &Ctx, mm: &ModelManager, id: i64) -> Result<E>
where
	MC: DbBmc,
	E: for<'r> FromRow<'r, PgRow> + Unpin + Send,
	E: HasFields,
{
	let db = mm.db();

	// -- Build query
	let mut query = Query::select();
	query
		.from(MC::table_ref())
		.columns(E::field_column_refs())
		.and_where(Expr::col(CommonIden::Id).eq(id));

	// -- Exec query
	let (sql, values) = query.build_sqlx(PostgresQueryBuilder);
	let entity = sqlx::query_as_with::<_, E, _>(&sql, values)
		.fetch_optional(db)
		.await?
		.ok_or(Error::EntityNotFound {
			entity: MC::TABLE,
			id,
		})?;

	Ok(entity)
}

pub async fn list<MC, E, F>(
	_ctx: &Ctx,
	mm: &ModelManager,
	filter: Option<F>,
	list_options: Option<ListOptions>,
) -> Result<Vec<E>>
where
	MC: DbBmc,
	E: for<'r> FromRow<'r, PgRow> + Unpin + Send,
	E: HasFields,
	F: TryInto<Condition, Error = IntoSeaError>,
{
	let db = mm.db();

	// -- Build the query
	let mut query = Query::select();
	query.from(MC::table_ref()).columns(E::field_column_refs());

	// condition from filter
	if let Some(filter) = filter {
		let cond: Condition = filter.try_into()?;
		query.cond_where(cond);
	}

	// list options
	if let Some(list_options) = list_options {
		list_options.apply_to_sea_query(&mut query);
	}

	// -- Execute the query
	let (sql, values) = query.build_sqlx(PostgresQueryBuilder);
	let entities = sqlx::query_as_with::<_, E, _>(&sql, values)
		.fetch_all(db)
		.await?;
	Ok(entities)
}

pub async fn update<MC, E>(
	ctx: &Ctx,
	mm: &ModelManager,
	id: i64,
	data: E,
) -> Result<()>
where
	MC: DbBmc,
	E: HasFields,
{
	let db = mm.db();

	let mut fields = data.not_none_fields();
	add_timestamps_for_update(&mut fields, ctx.user_id());
	let fields = fields.for_sea_update();

	// -- Build query
	let mut query = Query::update();
	query
		.table(MC::table_ref())
		.values(fields)
		.and_where(Expr::col(CommonIden::Id).eq(id));

	// -- Execute query
	let (sql, values) = query.build_sqlx(PostgresQueryBuilder);
	let count = sqlx::query_with(&sql, values)
		.execute(db)
		.await?
		.rows_affected();

	// -- Check result
	if count == 0 {
		Err(Error::EntityNotFound {
			entity: MC::TABLE,
			id,
		})
	} else {
		Ok(())
	}
}

pub async fn delete<MC>(_ctx: &Ctx, mm: &ModelManager, id: i64) -> Result<()>
where
	MC: DbBmc,
{
	let db = mm.db();

	// -- Build query
	let mut query = Query::delete();
	query
		.from_table(MC::table_ref())
		.and_where(Expr::col(CommonIden::Id).eq(id));

	// -- Execute query
	let (sql, values) = query.build_sqlx(PostgresQueryBuilder);
	let count = sqlx::query_with(&sql, values)
		.execute(db)
		.await?
		.rows_affected();

	// -- Check result
	if count == 0 {
		Err(Error::EntityNotFound {
			entity: MC::TABLE,
			id,
		})
	} else {
		Ok(())
	}
}

// region:    --- Utils
/// Update the timestamps info for create
/// (e.g., cid, ctime, and mid, mtime will be updated with the same values)
pub fn add_timestamps_for_create(fields: &mut Fields, user_id: i64) {
	let now = now_utc();
	fields.push(Field::new(TimestampIden::Cid.into_iden(), user_id.into()));
	fields.push(Field::new(TimestampIden::Ctime.into_iden(), now.into()));

	fields.push(Field::new(TimestampIden::Mid.into_iden(), user_id.into()));
	fields.push(Field::new(TimestampIden::Mtime.into_iden(), now.into()));
}

/// Update the timestamps info only for update.
/// (.e.g., only mid, mtime will be udpated)
pub fn add_timestamps_for_update(fields: &mut Fields, user_id: i64) {
	let now = now_utc();
	fields.push(Field::new(TimestampIden::Mid.into_iden(), user_id.into()));
	fields.push(Field::new(TimestampIden::Mtime.into_iden(), now.into()));
}
// endregion: --- Utils
