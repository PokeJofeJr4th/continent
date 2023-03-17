import copy
import random

table = {}
order = 2

input_data = {"Elf": "Adran Aelar Aerdeth Ahvain Aramil Arannis Aust Azaki Beiro Berrian Caeldrim Carric Dayereth "
                     "Dreali Efferil Eiravel Enialis Erdan Erevan Fivin Galinndan Gennal Hadarai Halimath Heian Himo "
                     "Immeral Ivellios Korfel Lamlis Laucian Lucan Mindartis Naal Nutae Paelias Peren Quarion Riardon "
                     "Rolen Soveliss Suhnae Thamior Tharivol Theren Theriatis Thervan Uthemar Vanuath Varis Adrie "
                     "Ahinar Alethaea Anastrianna Andraste Antinua Arara Baelitae Bethrynna Birel Caelynn Chaedi "
                     "Claira Dara Drusilia Elama Enna Faral Felosial Hatae Ielenia Ilanis Irann Jarsali Jelenneth "
                     "Keyleth Leshanna Lia Maiathah Malquis Meriele Mialee Myathethil Naivara Quelenna Quillathe "
                     "Ridaro Sariel Shanairla Shava Silaqui Sumnes Theirastra Thiala Tiaathque Traulam Vadania "
                     "Valanthe Valna Xanaphia",
              "Dwarf": "Anbera Artin Audhild Balifra Barbena Bardryn Bolhild Dagnal Dariff Delre Diesa Eldeth Eridred "
                       "Falkrunn Fallthra Finellen Gillydd Gunnloda Gurdis Helgret Helja Hlin Ilde Jarana Kathra Kilia "
                       "Kristryd Liftrasa Marastyr Mardred Morana Nalaed Nora Nurkara Oriff Ovina Riswynn Sannl "
                       "Therlin Thodris Torbera Tordrid Torgga Urshar Valida Vistra Vonana Werydd Whurdred Yurgunn "
                       "Adrik Alberich Baern Barendd Beloril Brottor Dain Dalgal Darrak Delg Duergath Dworic Eberk "
                       "Einkil Elaim Erias Fallond Fargrim Gardain Gilthur Gimgen Gimurt Harbek Kildrak Kilvar Morgran "
                       "Morkral Naldral Nordak Nuraval Oloric Olunt Orsik Oskar Rangrim Reirak Rurik Taklinn Thoradin "
                       "Thorin Thradal Tordek Traubon Travok Ulfgar Uraim Veit Vonbin Vondal Whurbin Urist Bembul "
                       "Tirist",
              "Orc": "Arha Baggi Bendoo Bilga Brakka Creega Drenna Ekk Emen Engong Fistula Gaaki Gorga Grai Greeba "
                     "Grigi Gynk Hrathy Huru Ilga Kabbarg Kansif Lagazi Lezre Murgen Murook Myev Nagrette Neega Nella "
                     "Nogu Oolah Ootah Ovak Ownka Puyet Reeza Shautha Silgre Sutha Tagga Tawar Tomph Ubada Vanchu Vola "
                     "Volen Vorka Yevelda Zagga",
              "Goblin": "Aaspar Aguus Belaluur Denaal Draraar Duusha Ekhaas Eluun Graal Gaduul Hashak Jheluum Kelaal "
                        "Mulaan Nasree Raleen Razu Rekseen Senen Shedroor Tajiin Tuneer Valii Wuun Aruget Chetiin "
                        "Daavn Dabrak Dagii Drevduul Duulan Fenic Gudruun Haluun Haruuc Jhazaal Kallaad Krakuul "
                        "Krootad Mazaan Munta Nasaar Rakari Reksit Tariic Taruuzh Thuun Vanii Vanon Wuudaraj Atcha "
                        "Draal Gazhaak Karthoon Khraal Kol Muut Paluur Rhukaan Shaarat Suthar Taarn Tor Volaar Darguun "
                        "Droaam Torlaac Olkhaan Arthuun Yarkuun Arashuul Dar Ghaal Golin Guul",
              "Metal": "Gold Silver Nickel Copper Lead Tin Pewter Bronze Iron Steel Atium Aluminum Duralumin Zinc "
                       "Bismuth Cobalt Osmium Iridium Platinum Mercury Uranium Chromium Stellite Billon Brass Electrum "
                       "Solder Amalgam Argentium Metal Conductivity Forge Alloy Forge",
              "Gemstone": "Agate Amethyst Opal Pearl Jet Tourmaline Ruby Diamond Sapphire Lapis Jade Quartz Moonstone "
                          "Topaz Onyx Malachite Pyrite Citrine Peridot Aquamarine Zircon Beryl Sunstone Ametrine "
                          "Alexandrite Emerald Spinel Adamantine Azurite Heliodor Calcite Cassiterite Cinnabar Cryolite "
                          "Moonstone Corundum Feldspar Pyrope Spessartine Gypsum Kaolinite Jasper Hematite",
              "Plant": "Apple Pear Peach Twig Leaf Branch Root Tree Bulb Fruit Flower Orange Pineapple Grass Grain "
                       "Nightshade Aspen Redwood Spruce Acacia Tomato Cactus Vegetable Rose Oak Birch Sunflower "
                       "Orchid Hemp Aloe Potato Corn Hydrangea Cucumber Hibiscus Lily Lavender Basil Flytrap Lemon "
                       "Garlic Tulip Coconut Ginger Fig Wheat Grape Vine Melon Watermelon Pepper Pine Evergreen "
                       "Deciduous Rice Citrus Bark Tulip",
              "Monster": "Leviathan Dragon Drake Loch Ness Kraken Titan Giant Beast Owlbear Displacer Wendigo Ghost "
                         "Ghast Monster Cryptid Yeti Lycanthrope Werewolf Beholder Tarrasque Behemoth Colossus Mammoth "
                         "Brute Fiend Ogre Devil Demon Bogeyman Savage Villain Dinosaur Serpent Bigfoot Chupacabra "
                         "Mothman Thunderbird",
              "Magic": "Wizardry Sorcery Hemalurgy Allomancy Feruchemy Alchemy Druidcraft Devotion Pyromancy Bending "
                       "Transmutation Evocation Divination Conjuration Illusion Enchantment Necromancy Gravitation "
                       "Surgebinding Cryomancy Electricity Magnetism Chemistry Voodoo Incantation Occult Supernatural "
                       "Thaumaturgy Algebra Calculus Geometry Witchcraft Spellcasting Deception Mathematics Artistry "
                       "Religion Adhesion Tension Capacitance",
              "Animal": "Bear Deer Rabbit Owl Wolf Bison Buffalo Warthog Pig Hog Lion Tiger Elephant Giraffe Shark "
                        "Whale Squonk Bee Bird Bat Scorpion Spider Cricket Grasshopper Cow Horse Mule Donkey Dog Cat "
                        "Panther Ocelot Dolphin Squid Snake Lemur Chimpanzee Zebra Orangutan Gorilla Panda Squirrel "
                        "Chipmunk Fox Sheep Bull Steer Frog Toad Raccoon Mammoth Sabertooth Beaver Coyote Moose "
                        "Antelope Octopus",
              "Organ": "Heart Skull Head Lung Liver Kidney Arm Leg Foot Hand Finger Toe Knuckle Artery Vein Torso Ribcage "
                       "Fingernail Toenail Skin Wrinkle Ankle Elbow Hip Waist Chest Femur Intestine Colon Esophagus "
                       "Mouth Tooth Tongue Gum Tonsil Throat Nerve Vertebra Back Hair Eardrum Neck Scalp Eyebrow "
                       "Nose Nostril Sinus Radius Ulna Carpal Tarsal Heel Back Bone Muscle Petcoral Abdomen Beard "
                       "Moustache"}
data_tables = {}
human_data = {"Start": []}


def load(s):
    if s == "":
        return
    table["Start"].append(s[:order])
    human_data["Start"].append(s[:order])
    for i in range(len(s) - order):
        try:
            table[s[i:i + order]]
        except KeyError:
            table[s[i:i + order]] = []
        table[s[i:i + order]].append(s[i + order])
        try:
            human_data[s[i:i + order]]
        except KeyError:
            human_data[s[i:i + order]] = []
        human_data[s[i:i + order]].append(s[i + order])


def generate(type, start=None, max_length=12, min_length=6):
    if start is None:
        s = random.choice([*data_tables[type]["Start"]])
    else:
        s = start
    try:
        while len(s) < max_length:
            options = []
            if s[-order:] in data_tables[type].keys():
                options = [*options, *data_tables[type][s[-order:]], *data_tables[type][s[-order:]], *data_tables[type][s[-order:]], *data_tables[type][s[-order:]], *data_tables[type][s[-order:]]]
            if s[-order:] in data_tables["Human"].keys():
                options = [*options, *data_tables["Human"][s[-order:]]]
            s += random.choice(options)
    except IndexError:
        pass
    if len(s) == max_length or len(s) < min_length:
        return generate(type, start, max_length)
    return s[0].upper() + s[1:-1]


for race in input_data.keys():
    table = {"Start": []}
    for word in input_data[race].lower().split(" "):
        load(word + ";")
    data_tables[race] = copy.deepcopy(table)

data_tables["Human"] = copy.deepcopy(human_data)
