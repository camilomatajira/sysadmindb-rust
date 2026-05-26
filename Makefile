
export DATABASE_URL:=sqlite://$(shell pwd)/mydb.db
export RUST_LOG=tower_http=debug


init_db:
	echo $(DATABASE_URL)
	sqlx migrate run

prepare_db:
	cargo sqlx prepare

run:
	cargo run

