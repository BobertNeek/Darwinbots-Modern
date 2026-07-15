# Live Controls, Toroidal World, and Ecology Design

Date: 2026-07-14
Status: Approved

The running simulation rail becomes a functional control surface without redesigning the setup wizard. It exposes live world, environment, physics, sensing, energy, history, statistics, and experiment controls while preserving the DB2-style layout.

Worlds are toroidal by default. CPU and GPU movement and projectiles wrap across opposite edges while preserving velocity. The viewport marks linked edges subtly. Left-dragging a bot previews and commits a direct move; right and middle drag continue to pan.

Waste remains part of simulation state, DNA, saves, and inspection. A render toggle only hides waste projectiles. Mutation sliders use observable two-way values. DNA remains the sole authority for ordinary reproduction, including .repro and .mrepro resource percentages.

The temporary unconditional one-energy metabolism tax defaults to zero. Movement, shots, DNA actions, and other explicit costs remain. DB2 plant defaults stay visible and live-editable. Historical bot DNA comes unmodified from the official Darwinbots2 repository.
