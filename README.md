# Rust Software Renderer

A modular CPU-based software rasterizer developed in Rust
for the Computer Graphics course at Universidad del Valle
de Guatemala (UVG).

## Overview

This repository provides the foundation for a software
rendering pipeline implemented primarily in Rust.

Development is organized into independent laboratory
branches, with the main branch reserved for stable
foundations and future project integration.

## Laboratory 4 — OBJ Model Loading

Planned features:

- Load triangulated OBJ models.
- Parse vertices and face indices.
- Assemble triangles from indexed geometry.
- Render triangle outlines using custom algorithms.
- Display the model through a framebuffer.
- Center and scale the model appropriately.

Branch: `lab4/obj-wireframe`

## Laboratory 5 — Triangle Filling

Planned features:

- Wireframe rendering.
- Barycentric triangle rasterization.
- Scanline triangle rasterization.
- Pixel coverage comparisons.
- Performance benchmarks.
- Profiling and analysis.

Branch: `lab5/triangle-filling`

## Architecture

The application follows a modular layered architecture.

Main components:

- Geometry
- Model loading
- Graphics pipeline
- Rasterization
- Fragment processing
- Framebuffer rendering

## Technology

- Rust
- CPU-based software rasterization
- Raylib for window management and framebuffer presentation

The graphics algorithms will be implemented within
the project's own rendering pipeline.

## Development

The project follows an incremental development process
with independent Git branches, automated tests and
technical documentation.

## Running

To execute the initial application:

    cargo run

To execute the tests:

    cargo test

## Current Status

Initial repository foundation.

The final requirements for Project 1 have not yet
been published.
