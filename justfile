# List available recipes
default:
    @just --list

# Run the backend Axum server
backend *args="serve":
    cargo run --manifest-path backend/Cargo.toml -- {{args}}

alias server := backend

# Run the frontend Vite development server
frontend:
    npm --prefix frontend run dev

alias web := frontend

# Provision the NLECloud project by name (idempotent, defaults to $PROJECT_NAME or smart-home-dock)
provision *args="":
    cargo run --manifest-path backend/Cargo.toml -- provision {{args}}

# Delete the NLECloud project by name (defaults to $PROJECT_NAME or smart-home-dock)
delete-project *args="":
    cargo run --manifest-path backend/Cargo.toml -- delete {{args}}

alias delete := delete-project
