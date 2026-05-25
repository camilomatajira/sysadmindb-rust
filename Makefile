
export DATABASE_URL:=sqlite://$(shell pwd)/mydb.db

init_db:
	echo $(DATABASE_URL)
	sqlx migrate run

prepare_db:
	cargo sqlx prepare

run:
	cargo run

