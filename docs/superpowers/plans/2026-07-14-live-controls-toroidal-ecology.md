# Live Controls, Toroidal World, and Ecology Implementation Plan

> **Status: Completed and archived.** This file preserves the original implementation sequence. Use `modern/docs/verification.md` and `docs/parity/gui-parity-report.md` for current evidence.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans.

**Goal:** Make the live simulation rail functional, add toroidal movement and bot dragging, repair mutation configuration, and restore viable DNA-driven ecology.

**Architecture:** Keep UI-only rendering preferences in WorldViewport; route live simulation settings through the existing immutable EnvironmentUpdate command; keep authoritative movement and reproduction in Rust. Historical DNA remains byte-for-byte source material.

**Tech Stack:** .NET 10, Avalonia, Rust, wgpu, WGSL.

## Tasks

- [x] Make mutation rows observable and two-way.
- [x] Replace static live-rail labels with focused controls and summaries.
- [x] Add a default-on toroidal setting across setup, FFI, CPU, GPU, and projectiles.
- [x] Add direct left-drag organism movement while preserving viewport panning.
- [x] Add waste and selected-vision render toggles.
- [x] Remove the temporary unconditional metabolism tax by default while retaining live tuning.
- [x] Bundle a primary-source historical bot compatibility pack.
- [ ] Build, run focused tests, package, and compare a screenshot to the approved blueprint.
