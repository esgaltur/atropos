.PHONY: setup db migrate run test check clean lint

# Load .env if it exists
ifneq ("$(wildcard .env)","")
    include .env
    export $(shell sed 's/=.*//' .env)
endif

setup: db migrate

db:
	docker-compose up -d

migrate:
	sqlx migrate run

run:
	cargo run

test:
	cargo test

check:
	cargo check

lint:
	cargo fmt --all -- --check
	cargo clippy -- -D warnings

clean:
	cargo clean
	docker-compose down
