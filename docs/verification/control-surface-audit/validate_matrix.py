#!/usr/bin/env python3
import json, sys
from pathlib import Path
matrix = Path(__file__).parents[1] / '..' / 'parity' / 'control-surface-matrix.json'
matrix = matrix.resolve()
data=json.loads(matrix.read_text())
allowed={'confirmed-parity','source-parity','intentional-difference','provisional-wine-result','broken','missing','blocked'}
required=['id','surface','label','control_type','visibility','enabled_conditions','modern_handler_or_binding','state_affected','original_db2_equivalent','original_default','modern_default','original_range_or_options','modern_range_or_options','persistence','live_update','interactive_test_id','original_screenshot','modern_screenshot','result','defect_severity','notes','status']
errors=[]
for i,row in enumerate(data):
    for k in required:
        if k not in row: errors.append(f'row {i} missing {k}')
    if row.get('status') not in allowed: errors.append(f"row {i} invalid status {row.get('status')}")
if len(data) < 55: errors.append(f'expected at least 55 controls, found {len(data)}')
if errors:
    print('\n'.join(errors)); sys.exit(1)
print(f'validated {len(data)} control-surface rows')
