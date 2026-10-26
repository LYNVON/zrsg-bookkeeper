import re

p = r'C:\Users\93783\XiaomiMiMoProjects\zrsj-bookkeeper\app\src\App.tsx'
t = open(p, encoding='utf-8').read()
new = open(r'C:\Users\93783\XiaomiMiMoProjects\zrsj-bookkeeper\app\src\home_page.txt', encoding='utf-8').read()
start = t.index('function Home({')
end = t.index('function ArList(')
t = t[:start] + new + '\n\n' + t[end:]
open(p, 'w', encoding='utf-8').write(t)
print('home spliced ok')
