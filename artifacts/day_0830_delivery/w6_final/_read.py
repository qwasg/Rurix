import json
d=json.load(open(r'artifacts/day_0830_delivery/w6_final/W6_GATES.json',encoding='utf-8'))
print('fails',d['fails'])
for r in d['rows']:
    if not r['pass']:
        print('FAIL',r['step'],r['wall_s'],'s')
        for line in (r.get('tail') or [])[-6:]: print('   ',line)
