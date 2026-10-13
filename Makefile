# Machine-local paths live in `.env` (see `.env.example`).
ROOT := $(abspath $(dir $(lastword $(MAKEFILE_LIST))))
ifneq (,$(wildcard $(ROOT)/.env))
include $(ROOT)/.env
export
endif

PROFILE ?= play
TEXTURE ?= humandiff
CARGO = cargo

.PHONY: help build run info stats check fmt clippy test clean

help:
	@echo "make build              build the workspace ($(PROFILE) profile)"
	@echo "make run [TEXTURE=x]    start exarust (first light)"
	@echo "make info               installation and pack inventory"
	@echo "make stats PACK=Objlib  formats inside a pack"
	@echo "make check              fmt check + clippy + tests"

build:
	$(CARGO) build --workspace --profile $(PROFILE)

run:
	$(CARGO) run -p exarust --profile $(PROFILE) -- $(TEXTURE)

info:
	$(CARGO) exa info

PACK ?= Resource
stats:
	$(CARGO) exa stats $(PACK)

check: fmt clippy test

fmt:
	$(CARGO) fmt --all -- --check

clippy:
	$(CARGO) clippy --workspace --all-targets -- -D warnings

test:
	$(CARGO) test --workspace

clean:
	$(CARGO) clean
