import os

translate = {
    'hyphen': '-',
    'hash': '#',
    'forwardslash': '/',
    'colon': ':',
    'percent': '%',
    'ampersand': '@',
    'dollar': '$',
    '1': '1',
    '2': '2',
    '3': '3',
    '4': '4',
    'exclamation': '!',
    'question': '?',
    'equals': '=',
    'asterix': '*',
    'caret': '^',
    'semicolon': ';',
    'lessthan': '<',
    'at': '@',
    'bar': '|',
    'backslash': '\\\\',
    'plus': '+',
    'tilde': '~',
}

def display(key, seq, path):
    print("""[[sound]]
key = "%(key)s"
seq = %(seq)s
path = "%(path)s"
    """ % {'key': r"%s" % key, 'seq': seq, 'path': path})


if __name__ == '__main__':
    base = 'assets/sounds/foxdot'
    inv_translate = dict([(v,k) for k, v in translate.items()])

    for root, directory, files in os.walk(base):
        #print(root, directory, files, os.path.relpath(root, start=base))
        path = os.path.relpath(root, start=base)
        parts = path.split("/")
        # print(parts)
        for seq, f in enumerate(sorted(files)):
            if parts[0] == '_' and parts[1] in translate:
                v = translate[parts[1]]
                display(v, seq, os.path.join(base,path, f))
            elif len(parts) == 2 and parts[1] == 'lower':
                v = parts[0]
                display(v, seq, os.path.join(base,path, f))
            elif len(parts) == 2 and parts[1] == 'upper':
                v = parts[0].upper()
                display(v, seq, os.path.join(base,path, f))


                 
