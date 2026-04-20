# Layout Tightening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Tighten the main client layout so the temperature ring, right-side controls, and footer feel balanced and fully occupied.

**Architecture:** Keep the change isolated to `src/App.tsx`. Rework the center temperature cluster into two independent layers, compress the right control column into a header plus a compact status strip, and normalize top/footer heights so the window reads like a single polished desktop surface.

**Tech Stack:** React, Tailwind CSS, motion/react, Tauri window APIs.

---

### Task 1: Rework the center ring

**Files:**
- Modify: `src/App.tsx`

- [ ] **Step 1: Update the ring structure**

Move the label/mode badge into an absolutely positioned `ring-inner` container pinned near the top of the circle, and place the temperature number inside a separate `temp-center` container that fills the circle and centers the value independently.

- [ ] **Step 2: Keep the circle interactive**

Ensure the up/down buttons still sit on either side of the number and remain clickable after the layer split.

### Task 2: Tighten the main and right column spacing

**Files:**
- Modify: `src/App.tsx`

- [ ] **Step 1: Expand the main layout**

Change the main content wrapper to `justify-between` with horizontal padding of `48px` so the left circle and right controls feel anchored to the sides.

- [ ] **Step 2: Compact the right control column**

Add a bottom divider to the right-side header, then replace the tall spacing with a compact `status-mini` block before the switches.

### Task 3: Normalize top and bottom bars

**Files:**
- Modify: `src/App.tsx`

- [ ] **Step 1: Set the title bar height to 40px**

Use a fixed-height top bar so the client shell feels deliberate rather than padded.

- [ ] **Step 2: Set the footer height to 36px**

Shrink the bottom bar to a consistent 36px status strip and keep the scrolling telemetry inside it.
