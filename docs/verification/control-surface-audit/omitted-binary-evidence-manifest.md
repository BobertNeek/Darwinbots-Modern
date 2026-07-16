# Omitted Binary Evidence Manifest

Rendered screenshots are intentionally kept out of Git because Codex Create PR does not support binary files in the diff. They remain in the cloud workspace after `git rm --cached` and can be regenerated with:

```bash
./docs/verification/control-surface-audit/run-gui-audit.sh
```

The command above launches Xvfb/Openbox, builds Darwinbots Modern, drives the rendered Avalonia GUI with `xdotool`, and writes screenshots under `docs/verification/control-surface-audit/modern/`.

| Omitted file | SHA-256 | Regeneration command |
|---|---|---|
| `docs/verification/control-surface-audit/modern/live-add-obstacle-modern-020.png` | `5a6f50fe86dc7771a5fbca126602912fe17c76aac685126c2ff0f9ff8594f94c` | `./docs/verification/control-surface-audit/run-gui-audit.sh` |
| `docs/verification/control-surface-audit/modern/live-add-teleporter-modern-021.png` | `5a6f50fe86dc7771a5fbca126602912fe17c76aac685126c2ff0f9ff8594f94c` | `./docs/verification/control-surface-audit/run-gui-audit.sh` |
| `docs/verification/control-surface-audit/modern/live-apply-energy-modern-024.png` | `5a6f50fe86dc7771a5fbca126602912fe17c76aac685126c2ff0f9ff8594f94c` | `./docs/verification/control-surface-audit/run-gui-audit.sh` |
| `docs/verification/control-surface-audit/modern/live-dna-editor-modern-027.png` | `7b6b9f5ae366c4849c388fe7c9646c8435018487866fc89b4f806ef874b98b68` | `./docs/verification/control-surface-audit/run-gui-audit.sh` |
| `docs/verification/control-surface-audit/modern/live-initial-modern-013.png` | `fe734cedeea8e478fad22df40f64c5a6c574d754a8cce0748c29743b2e792e91` | `./docs/verification/control-surface-audit/run-gui-audit.sh` |
| `docs/verification/control-surface-audit/modern/live-open-physics-modern-023.png` | `5a6f50fe86dc7771a5fbca126602912fe17c76aac685126c2ff0f9ff8594f94c` | `./docs/verification/control-surface-audit/run-gui-audit.sh` |
| `docs/verification/control-surface-audit/modern/live-pause-modern-015.png` | `5a6f50fe86dc7771a5fbca126602912fe17c76aac685126c2ff0f9ff8594f94c` | `./docs/verification/control-surface-audit/run-gui-audit.sh` |
| `docs/verification/control-surface-audit/modern/live-remove-feature-modern-022.png` | `5a6f50fe86dc7771a5fbca126602912fe17c76aac685126c2ff0f9ff8594f94c` | `./docs/verification/control-surface-audit/run-gui-audit.sh` |
| `docs/verification/control-surface-audit/modern/live-render-waste-toggle-modern-019.png` | `5a6f50fe86dc7771a5fbca126602912fe17c76aac685126c2ff0f9ff8594f94c` | `./docs/verification/control-surface-audit/run-gui-audit.sh` |
| `docs/verification/control-surface-audit/modern/live-run-modern-014.png` | `5a6f50fe86dc7771a5fbca126602912fe17c76aac685126c2ff0f9ff8594f94c` | `./docs/verification/control-surface-audit/run-gui-audit.sh` |
| `docs/verification/control-surface-audit/modern/live-step-modern-016.png` | `5a6f50fe86dc7771a5fbca126602912fe17c76aac685126c2ff0f9ff8594f94c` | `./docs/verification/control-surface-audit/run-gui-audit.sh` |
| `docs/verification/control-surface-audit/modern/live-toroidal-toggle-modern-018.png` | `5a6f50fe86dc7771a5fbca126602912fe17c76aac685126c2ff0f9ff8594f94c` | `./docs/verification/control-surface-audit/run-gui-audit.sh` |
| `docs/verification/control-surface-audit/modern/live-turbo-modern-017.png` | `5a6f50fe86dc7771a5fbca126602912fe17c76aac685126c2ff0f9ff8594f94c` | `./docs/verification/control-surface-audit/run-gui-audit.sh` |
| `docs/verification/control-surface-audit/modern/live-viewport-drag-modern-026.png` | `5a6f50fe86dc7771a5fbca126602912fe17c76aac685126c2ff0f9ff8594f94c` | `./docs/verification/control-surface-audit/run-gui-audit.sh` |
| `docs/verification/control-surface-audit/modern/live-viewport-select-modern-025.png` | `5a6f50fe86dc7771a5fbca126602912fe17c76aac685126c2ff0f9ff8594f94c` | `./docs/verification/control-surface-audit/run-gui-audit.sh` |
| `docs/verification/control-surface-audit/modern/setup-advanced-open-modern-005.png` | `2f951631fffbdcc046ffd93e98dd8def27a81cabb599c19ee044fec6c0d9f9df` | `./docs/verification/control-surface-audit/run-gui-audit.sh` |
| `docs/verification/control-surface-audit/modern/setup-backend-combo-modern-011.png` | `75c88f757fc502bd9f464d385ac2c0c3d881908eaebaa107b88c58bbaabfb621` | `./docs/verification/control-surface-audit/run-gui-audit.sh` |
| `docs/verification/control-surface-audit/modern/setup-create-world-before-modern-012.png` | `eb8e95495bc57f438a33fba436ee0f2d1fde62b6714a1a7b3b98e01f173e536f` | `./docs/verification/control-surface-audit/run-gui-audit.sh` |
| `docs/verification/control-surface-audit/modern/setup-initial-modern-001.png` | `7d3ae47318541d36d16b64562ba3453d1e2c1202cceee168df9db3e50fa2519f` | `./docs/verification/control-surface-audit/run-gui-audit.sh` |
| `docs/verification/control-surface-audit/modern/setup-menu-file-open-modern-002.png` | `7d3ae47318541d36d16b64562ba3453d1e2c1202cceee168df9db3e50fa2519f` | `./docs/verification/control-surface-audit/run-gui-audit.sh` |
| `docs/verification/control-surface-audit/modern/setup-menu-help-open-modern-004.png` | `7d3ae47318541d36d16b64562ba3453d1e2c1202cceee168df9db3e50fa2519f` | `./docs/verification/control-surface-audit/run-gui-audit.sh` |
| `docs/verification/control-surface-audit/modern/setup-menu-view-open-modern-003.png` | `7d3ae47318541d36d16b64562ba3453d1e2c1202cceee168df9db3e50fa2519f` | `./docs/verification/control-surface-audit/run-gui-audit.sh` |
| `docs/verification/control-surface-audit/modern/setup-mode-zerobots-modern-006.png` | `2f951631fffbdcc046ffd93e98dd8def27a81cabb599c19ee044fec6c0d9f9df` | `./docs/verification/control-surface-audit/run-gui-audit.sh` |
| `docs/verification/control-surface-audit/modern/setup-mode-zerobots-vegetables-modern-007.png` | `2f951631fffbdcc046ffd93e98dd8def27a81cabb599c19ee044fec6c0d9f9df` | `./docs/verification/control-surface-audit/run-gui-audit.sh` |
| `docs/verification/control-surface-audit/modern/setup-mutation-max-modern-010.png` | `2f951631fffbdcc046ffd93e98dd8def27a81cabb599c19ee044fec6c0d9f9df` | `./docs/verification/control-surface-audit/run-gui-audit.sh` |
| `docs/verification/control-surface-audit/modern/setup-mutation-mid-modern-009.png` | `2f951631fffbdcc046ffd93e98dd8def27a81cabb599c19ee044fec6c0d9f9df` | `./docs/verification/control-surface-audit/run-gui-audit.sh` |
| `docs/verification/control-surface-audit/modern/setup-mutation-min-modern-008.png` | `2f951631fffbdcc046ffd93e98dd8def27a81cabb599c19ee044fec6c0d9f9df` | `./docs/verification/control-surface-audit/run-gui-audit.sh` |
