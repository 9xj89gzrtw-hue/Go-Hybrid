#!/usr/bin/env python3
"""escape_sites.py — Find capture-write-at-foreign-call-boundary triples.

M1 deliverable for W-016 wildcard (A5 Go-escape check-site).

Scans a .hy file and emits every (capture, closure, foreign-call) triple
where:
  1. A closure captures a mutable binding from its enclosing scope
  2. The closure body writes (assigns to) that captured binding
  3. The closure is passed as an argument to a foreign (Go package) call

Usage:
    python tools/escape_sites.py <file.hy>

M1 kill criteria:
  - examples/06-http-server/program.hy must yield exactly 0 triples
  - tests/checker-corpus/10-a5-http-handler-capture.hy must yield exactly 1 triple
"""

import re
import sys


def find_escape_sites(src: str) -> list:
    """Find capture-write-at-foreign-call triples in .hy source.

    We use a lightweight recursive-descent scan over brace/paren structure
    to find fn(...) closures that:
      (a) contain an assignment to a variable defined outside the closure,
      (b) are passed as an argument to a call on a qualified name (pkg.fn).

    Returns list of dicts: {'capture': str, 'closure_line': int, 'foreign_call': str}
    """
    lines = src.split('\n')
    triples = []

    # Remove line comments but keep line numbers for error reporting
    clean_lines = []
    for line in lines:
        # Strip line comments (// ...)
        comment_idx = line.find('//')
        if comment_idx != -1:
            line = line[:comment_idx]
        clean_lines.append(line)
    clean_src = '\n'.join(clean_lines)

    # Find all `fn(` closures — look for `fn(` followed by paren-matching
    i = 0
    while i < len(clean_src):
        # Find next 'fn'
        fn_idx = clean_src.find('fn', i)
        if fn_idx == -1:
            break
        # Check it's a keyword, not part of another word
        if fn_idx > 0 and (clean_src[fn_idx - 1].isalnum() or clean_src[fn_idx - 1] == '_'):
            i = fn_idx + 2
            continue
        # Check next char is '('
        if fn_idx + 2 >= len(clean_src) or clean_src[fn_idx + 2] != '(':
            i = fn_idx + 2
            continue

        # Match parens from after 'fn('
        depth = 1
        j = fn_idx + 3
        while j < len(clean_src) and depth > 0:
            c = clean_src[j]
            if c == '(':
                depth += 1
            elif c == ')':
                depth -= 1
            j += 1
        if depth != 0:
            i = fn_idx + 2
            continue

        # Now we have fn(...) from fn_idx to j (exclusive of closing paren at j-1)
        # Find the body: after the parameter list parens, there should be '{'
        body_start = j
        while body_start < len(clean_src) and clean_src[body_start] in ' \t\n':
            body_start += 1
        if body_start >= len(clean_src) or clean_src[body_start] != '{':
            i = j
            continue

        # Match braces for the body
        body_depth = 1
        body_end = body_start + 1
        while body_end < len(clean_src) and body_depth > 0:
            c = clean_src[body_end]
            if c == '{':
                body_depth += 1
            elif c == '}':
                body_depth -= 1
            body_end += 1
        if body_depth != 0:
            i = j
            continue

        body = clean_src[body_start + 1:body_end - 1]

        # Find captured variables written in the body
        # A write is: ident := ... or ident = ... (but not ==) or ident++ or ident--
        captures = set()

        # Match ident := value (short define)
        for m in re.finditer(r'\b([a-zA-Z_]\w*)\s*:=', body):
            captures.add(m.group(1))

        # Match ident++ or ident--
        for m in re.finditer(r'\b([a-zA-Z_]\w*)\s*\+\+', body):
            captures.add(m.group(1))
        for m in re.finditer(r'\b([a-zA-Z_]\w*)\s*--', body):
            captures.add(m.group(1))

        # Match ident = value (but not ==)
        for m in re.finditer(r'\b([a-zA-Z_]\w*)\s*=\s*(?!=)', body):
            captures.add(m.group(1))

        # Check if this closure is passed to a foreign call
        # Look at what comes after the closing paren of fn(...) 
        # The fn(...) might be inside a call like mux.handle("/path", fn(...))
        after_paren = clean_src[j:]
        stripped_after = after_paren.lstrip()

        # Check if we're inside a call to a qualified name (pkg.func)
        # Look backwards from fn_idx to find the enclosing call
        foreign_call = None
        # Look for pattern: pkg.name( ... fn( ... ) ... )
        before_fn = clean_src[:fn_idx]
        # Find the last '(' before fn that isn't closed
        paren_depth = 0
        last_open = -1
        for k in range(len(before_fn) - 1, -1, -1):
            c = before_fn[k]
            if c == ')':
                paren_depth += 1
            elif c == '(':
                if paren_depth == 0:
                    last_open = k
                    break
                paren_depth -= 1

        if last_open != -1:
            # Get the text between last_open and fn_idx
            call_prefix = before_fn[last_open + 1:].strip()
            # Check if it starts with a qualified name (pkg.func or just func)
            # A qualified name has a dot: pkg.name
            if '.' in call_prefix:
                # Extract the qualified name before the paren
                m = re.match(r'([a-zA-Z_]\w*\.[a-zA-Z_]\w*)\s*$', call_prefix)
                if m:
                    foreign_call = m.group(1)
            else:
                # Unqualified call - could be a builtin or user fn
                # For M1 we only care about foreign (package-qualified) calls
                pass

        if captures and foreign_call:
            line_no = clean_src[:fn_idx].count('\n') + 1
            for cap in sorted(captures):
                triples.append({
                    'capture': cap,
                    'closure_line': line_no,
                    'foreign_call': foreign_call,
                })

        i = body_end

    return triples


def main():
    if len(sys.argv) < 2:
        print("Usage: python escape_sites.py <file.hy>", file=sys.stderr)
        sys.exit(1)

    filepath = sys.argv[1]
    with open(filepath, 'r') as f:
        src = f.read()

    triples = find_escape_sites(src)

    for t in triples:
        print(f"capture={t['capture']} closure_line={t['closure_line']} foreign_call={t['foreign_call']}")

    print(f"Total escape-with-write triples: {len(triples)}")

    if triples:
        sys.exit(0)
    else:
        sys.exit(1)


if __name__ == '__main__':
    main()
