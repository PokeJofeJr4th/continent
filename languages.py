import copy
import random
import re


class Definition:
    def __init__(self, definition, parent=None):
        self.children = []
        self.parent = parent
        self.definition = definition

    def add_child(self, definition):
        child = Definition(definition, self)
        self.children.append(child)
        return child

    def expand(self):
        if self.children:
            return [*(f"{self.definition}/{d}" for c in self.children for d in c.expand()), self.definition]
        else:
            return [self.definition]

    def __str__(self):
        if self.children:
            return f"{self.definition}: [{', '.join(str(c) for c in self.children)}]"
        return self.definition

    def __getitem__(self, item):
        if self.definition == item:
            return self
        path = item.split("/")
        for i in self.children:
            if i.definition == path[1]:
                return i["/".join(path[1:])]
        print(f"Could not find {item}")
        return None


thing = Definition("Thing")


def init(context):
    for k in context.keys():
        category = thing.add_child(k)
        for i in context[k]:
            category.add_child(i)
    person = thing.add_child("Person")
    for k in ["Human", "Elf", "Dwarf", "Goblin", "Orc", "Kobold"]:
        race = person.add_child(k)
        for c in ["Male", "Female", "Adult", "Young"]:
            race.add_child(f"{c} {k}")
    for k in ["City", "Road", "Sky", "Star", "Sun", "Moon", "Night", "Day", "Up", "Down"]:
        thing.add_child(k)
    print(thing)
    print(thing.expand())


def protolang():
    words = []
    while len(words) < 128:
        word = "".join((random.choice(["p", "t", "k", "n", "l", "r"]) + random.choice(["a", "e", "i", "o", "u"]))
                       for i in range(random.choice([1, 2, 2, 2, 2, 3, 3, 3])))
        if word in words:
            continue
        words.append(word)
    random.shuffle(words)
    dictionary = {}
    for d in thing.expand():
        if len(words) <= 0:
            break
        dictionary[d] = words.pop()
    return dictionary


def evolve(lang):
    cons = "[ptkbdgfsxvzGywrlmn]"
    vowel = "[aeioɔu]"
    liquid = "[ywrl]"
    plosive = "[ptkbdgfsþxvzG]"
    stop = "[ptkbdg]"
    fricative = "[fsþxvzG]"
    nasal = "[mn]"
    unvoiced = "[ptkfsþx]"
    words = [*lang.values()]
    random.shuffle(words)

    def iterate(word):
        return None

    for w in words:
        if re.search(f"{vowel}{cons}{vowel}{liquid}{vowel}", w):
            def iterate(word):  # apara => apra
                while True:
                    m = re.search(f"{vowel}{cons}{vowel}{liquid}{vowel}", word)
                    if m is None:
                        break
                    word = word[:m.span()[0] + 2] + word[m.span()[0] + 3:]
                return word
        elif re.search(f"{vowel}{liquid}{vowel}{cons}{vowel}", w):
            def iterate(word):  # arapa => arpa
                while True:
                    m = re.search(f"{vowel}{liquid}{vowel}{cons}{vowel}", word)
                    if m is None:
                        break
                    word = word[:m.span()[0] + 2] + word[m.span()[0] + 3:]
                return word
        elif re.search(f"{vowel}{nasal}{vowel}{plosive}{vowel}", w):
            def iterate(word):  # anapa => anpa
                while True:
                    m = re.search(
                        f"{vowel}{nasal}{vowel}{plosive}{vowel}", word)
                    if m is None:
                        break
                    word = word[:m.span()[0] + 2] + word[m.span()[0] + 3:]
                return word
        elif re.search(f"{vowel}{nasal}{vowel}{nasal}{vowel}", w):
            def iterate(word):  # anana => anna
                while True:
                    m = re.search(f"{vowel}{nasal}{vowel}{nasal}{vowel}", word)
                    if m is None:
                        break
                    word = word[:m.span()[0] + 2] + word[m.span()[0] + 3:]
                return word
        elif re.search(f"{vowel}{plosive}{vowel}{plosive}{vowel}", w):
            def iterate(word):  # arapa => arpa
                while True:
                    m = re.search(
                        f"{vowel}{plosive}{vowel}{plosive}{vowel}", word)
                    if m is None:
                        break
                    word = word[:m.span()[0] + 2] + word[m.span()[0] + 3:]
                return word
        elif re.search(f"{vowel}{stop}{stop}{vowel}", w):
            def iterate(word):  # arapa => arpa
                while True:
                    m = re.search(f"{vowel}{stop}{stop}{vowel}", word)
                    if m is None:
                        break
                    word = word[:m.span()[0] + 1] + {"p": "f", "b": "v", "t": "s", "d": "z", "k": "x", "g": "G"}[
                        word[m.span()[0] + 1]] + word[m.span()[0] + 2:]
                return word
        elif re.search("n[pbfv]", w):
            def iterate(word):  # anpa => ampa
                while True:
                    m = re.search("n[pbfv]", word)
                    if m is None:
                        break
                    word = word[:m.span()[0]] + "m" + word[m.span()[0] + 1:]
                return word
        elif re.search("m[tdsz]", w):
            def iterate(word):  # amta => anta
                while True:
                    m = re.search("m[tdsz]", word)
                    if m is None:
                        break
                    word = word[:m.span()[0] + 1] + "n" + \
                        word[m.span()[0] + 2:]
                return word
        elif re.search(f"{vowel}{unvoiced}{vowel}", w):
            def iterate(word):  # arapa => araba
                while True:
                    m = re.search(f"{vowel}{unvoiced}{vowel}", word)
                    if m is None:
                        break
                    word = word[:m.span()[0] + 1] + {"p": "b", "t": "d", "k": "g",
                                                     "f": "v", "s": "z", "x": "G"}[word[m.span()[0] + 1]] + word[m.span()[
                                                         0] + 2:]
                return word
        elif re.search(f"{vowel}[Gx]{cons}", w):
            if random.random() < 0.25:
                def iterate(word):  # axka => axa
                    while True:
                        m = re.search(f"{vowel}[Gx]{cons}", word)
                        if m is None:
                            break
                        word = word[:m.span()[0] + 2] + word[m.span()[0] + 3:]
                    return word
            elif random.random() < 0.6:
                letter = random.choice(["r", "y"])

                def iterate(word):  # axka => arka
                    while True:
                        m = re.search(f"{vowel}[Gx]{cons}", word)
                        if m is None:
                            break
                        word = word[:m.span()[0] + 1] + letter + \
                            word[m.span()[0] + 2:]
                    return word
            else:
                def iterate(word):  # axka => aka
                    while True:
                        m = re.search(f"{vowel}[Gx]{cons}", word)
                        if m is None:
                            break
                        word = word[:m.span()[0] + 1] + word[m.span()[0] + 2:]
                    return word
        elif random.random() < 0.4:
            def iterate(word):
                return word.replace("a", "%").replace("e", "a").replace("i", "e").replace("u", "i").replace("o", "u").replace("ɔ", "o").replace("%", "ɔ")
        elif random.random() < 0.4:
            def iterate(word):
                return word.replace("ɔ", "%").replace("o", "ɔ").replace("u", "o").replace("i", "u").replace("e", "i").replace("a", "e").replace("%", "a")
    else:
        if iterate("spaghetti") is None:
            print("No improvements available")
            return

    for k in lang.keys():
        # if lang[k] != iterate(lang[k]):
        #     print(f"{lang[k]} => {iterate(lang[k])}")
        lang[k] = iterate(lang[k])


if __name__ == "__main__":
    init({"Metal": ["Steel", "Iron", "Copper", "Gold"],
         "Plant": ["Tree", "Fruit"]})
    proto = protolang()
    proto2 = copy.deepcopy(proto)
    print(proto.values())
    for n in range(20):
        evolve(proto)
        evolve(proto2)
    print(proto.values())
    print(proto2.values())
