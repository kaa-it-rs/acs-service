VERSION := 0.7.0

check:
	cargo watch -x check -x test -x run

coverage:
	cargo tarpaulin --ignore-test

fmt:
	cargo fmt -- --check

security:
	cargo audit

expand:
	cargo expand

udeps:
	cargo +nightly udeps --all-targets

test:
	TEST_LOG=true cargo test | bunyan

debug_up:
	docker-compose -f docker-compose-debug.yml up -d

debug_down:
	docker-compose -f docker-compose-debug.yml down

debug_restart: debug_down debug_up

build:
	docker build -t "opener-service:${VERSION}" ./opener-service
	docker tag "opener-service:${VERSION}" "akruglov/opener-service:${VERSION}"
	docker tag "opener-service:${VERSION}" "akruglov/opener-service:latest"

push: build
	docker login
	docker image push "akruglov/opener-service:${VERSION}"
	docker image push "akruglov/opener-service:latest"

up:
	docker login
	docker-compose up -d	
