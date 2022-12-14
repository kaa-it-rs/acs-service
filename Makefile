VERSION := 1.0.28

watch_service:
	cargo watch -x 'check --color=always' -x 'test -- --color=always' -x 'run -p acs-service'

clippy:
	cargo clippy --all --all-features --tests -- -D warnings

coverage:
	cargo tarpaulin --ignore-test

fmt_check:
	cargo fmt -- --check

fmt:
	cargo fmt --all

security:
	cargo audit

expand:
	cargo expand

udeps:
	cargo +nightly udeps --all-targets

test:
	TEST_LOG=true cargo test | bunyan

debug_up:
	docker-compose -p acs-service-rs -f ./deployment/compose/docker-compose-debug.yml up -d

debug_down:
	docker-compose -p acs-service-rs -f ./deployment/compose/docker-compose-debug.yml down --remove-orphans

debug_restart: debug_down debug_up

build:
	docker build -t "acs-service:${VERSION}" ./opener-service
	docker tag "acs-service:${VERSION}" "akruglov/acs-service:${VERSION}"
	docker tag "acs-service:${VERSION}" "akruglov/acs-service:latest"

push: build
	docker login
	docker image push "akruglov/acs-service:${VERSION}"
	docker image push "akruglov/acs-service:latest"

up:
	docker login
	docker compose -p acs-service-rs -f ./deployment/compose/docker-compose.yml up -d

down:
	docker compose -p acs-service-rs -f ./deployment/compose/docker-compose.yml down

restart: down up

run_service:
	cargo run -p acs-service

  	
