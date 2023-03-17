import random
import names


MagicMaterial = ""
MagMatProps = ()
MagMatType = ""


def generate_magic():
    global MagicMaterial, MagMatProps, MagMatType
    print("Random Magic System:")
    # SOURCE
    if random.random() < 1:  # Resource-based
        localization = "ubiquitous"
        r_i = random.randint(0, 4)
        rarity = ["extremely rare", "very rare", "rare", "common", "very common"][r_i]
        if random.random() < 0.6:
            localization = "localized"
        MagMatType = random.choice(["Metal", "Plant", "Gemstone"])
        MagicMaterial = names.generate(MagMatType)
        if MagMatType in ["Metal", "Gemstone"]:
            MagMatProps = (1 if localization == "ubiquitous" else 6, 2 + r_i, 9)
        elif MagMatType == "Plant":
            MagMatProps = ((4 if localization == "ubiquitous" else 10) - r_i, 1, 9)
        print(f"Magic comes from " + MagicMaterial + f", a {localization} " +
              ("but" if localization == "ubiquitous" and r_i <= 2 else "and") + f" {rarity} " +
              MagMatType +
              (" found in a " + random.choice(["mountain.", "forest.", "plain.", "cave.", "sea."])
               if localization == "localized" else "."))
    else:  # Entity-based
        print("Magic comes from a powerful entity.")
    # USERS
    hereditary = random.random() < 0.6
    training = random.random() < 0.6
    user_string = ""
    if not hereditary and random.random() < 0.3:
        user_string += "Magic-users must enter a pact to gain powers. "
    if hereditary:
        user_string += "Access to magic is hereditary. "
    if training:
        user_string += "Magic requires training to use."
    if not hereditary and not training:
        user_string += "Anyone can become a magic-user."
    print(user_string)
    # ITEMS
    if random.random() < 0.4:
        print("There are Focuses that amplify magical effects.")
    if random.random() < 0.4:
        print("There are items that can store magical power for later use.")
    # EFFECTS
    effect_num = random.randint(4, 10)
    print(f"There are {effect_num} different magical effects that can be produced:")
    for n in range(1, effect_num + 1):
        effect_type = random.randint(0, 4)
        if effect_type == 0:  # Buff/Debuff
            print(f"\t{n}. " + random.choice(["Increase", "Decrease"]) + " the " +
                  random.choice(["strength", "stamina", "toughness", "speed", "perception", "age"]) + " of " +
                  random.choice(["yourself.", "anyone you touch.", "anyone you can see."]))
        elif effect_type == 1:  # Energy
            if random.random() < 0.7:
                print(f"\t{n}. " + random.choice(["Increase", "Decrease"]) + " the " +
                      random.choice(["luminosity", "temperature"]) + " of " +
                      random.choice(["yourself.", "an object you touch.", "an object you can see."]))
            else:
                print(f"\t{n}. Apply a" + random.choice([" force", " force", "n electric charge"]) + " to " +
                      random.choice(["yourself.", "an object you touch.", "an object you can see."]))
        elif effect_type == 2:  # Psychic
            print(f"\t{n}. " + random.choice(
                ["Influence the emotions of", "Create an illusion for", "Control the actions of"]) + " " +
                  random.choice(["anyone you touch.", "anyone you can see.", "anyone near you"]))
        elif effect_type == 3:  # Metamagic
            print(f"\t{n}. " + random.choice(["Prevent", "Weaken", "Strengthen", "Detect"]) + " magic from " +
                  random.choice(["anyone", "anyone you can see", "anyone you touch", "anyone near you"]) +
                  " that targets " +
                  random.choice(["you.", "anyone you touch.", "anyone you can see.", "anyone near you."]))
        elif effect_type == 4:  # Divination
            print(f"\t{n}. Find the " + random.choice(["true", "possible"]) + " " +
                  random.choice(["distant past", "immediate past", "present", "immediate future", "distant future"]) +
                  " of " + random.choice(["yourself.", "anyone you touch.", "anyone near you.", "anyone you can see.",
                                          "anyone, as long as you have an item that is strongly connected to them."]))
