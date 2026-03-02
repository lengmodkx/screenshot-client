# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A Windows desktop screenshot client for school smart board devices. Takes timed screenshots and uploads to cloud or saves locally.

## Commands

```bash
# Development
npm run tauri dev          # Run in development mode (frontend + Rust backend)
npm run dev                # Frontend only (Vite dev server)

# Build
npm run tauri build        # Build production release (exe in src-tauri/target/release/)
npm run build              # Frontend build only
```

## Architecture

**Tech Stack:**
- Tauri 2.x (desktop framework)
- React 19 + TypeScript + Tailwind CSS 4 + Vite 7
- Rust backend

**Key Structure:**
- `src/` - React frontend (App.tsx contains all UI logic)
- `src-tauri/src/lib.rs` - Rust backend with all Tauri commands
- `src-tauri/tauri.conf.json` - Tauri configuration

**Rust Backend (lib.rs):**
- `capture_screen` - Captures screenshot using `screenshots` crate
- `save_screenshot_to_local` - Saves PNG to local path
- `upload_screenshot` - POST multipart to cloud API with Bearer token
- `login/logout` - Authentication with token storage
- `get_config/update_config` - JSON config management (stored in `%APPDATA%/ScreenshotClient/config.json`)
- `cleanup_old_files` - Deletes screenshots older than retention_days
- `check_network` - Health check API endpoint

**Frontend (App.tsx):**
- Uses `setInterval` for timed screenshot based on `config.interval`
- Automatically falls back to local save if upload fails
- Checks network every 30 seconds

## Config Location

`%APPDATA%/ScreenshotClient/config.json`

| Field | Description |
|-------|-------------|
| interval | Screenshot interval in seconds (default: 10) |
| mode | "cloud" or "local" |
| local_path | Local save directory |
| api_url | API server URL |
| token | Auth token (set after login) |
| username | Logged in username |
| retention_days | Days to keep local screenshots (default: 7) |

## API Integration

The app expects these endpoints on the configured `api_url`:
- `POST /api/login` - Body: `{username, password}`, Response: `{token}`
- `POST /api/screenshot/upload` - Header: `Authorization: Bearer <token>`, Body: multipart image file
- `GET /api/health` - Health check (optional)
