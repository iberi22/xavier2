import sys
with open(r'E:\scripts-python\xavier2\src\memory\sqlite_vec_store.rs', 'r', encoding='utf-8') as f:
    content = f.read()
lines = content.split('\n')
fixed = 0
for i, line in enumerate(lines):
    stripped = line.rstrip('\r')
    # The broken line: 8 spaces + quote+close+hash+question+semicolon (5 visible chars)
    # Should be: 8 spaces + quote+hash+close+question+semicolon (6 visible chars)
    if stripped == '        ")#?;':
        print(f'Found broken line at {i+1}: {repr(stripped)}')
        lines[i] = '        "#)?;'
        fixed += 1
    elif stripped == '        ")#?;'.replace('\r',''):
        print(f'Found CR-broken line at {i+1}: {repr(stripped)}')
        lines[i] = '        "#)?;'
        fixed += 1
print(f'Fixed {fixed} occurrences')
with open(r'E:\scripts-python\xavier2\src\memory\sqlite_vec_store.rs', 'w', encoding='utf-8') as f:
    f.write('\n'.join(lines))
print('Done')
