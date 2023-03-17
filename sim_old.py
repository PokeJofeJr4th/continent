import pygame
from pygame.locals import *
from jinja2 import Environment, FileSystemLoader
import sys
import random
import math
from statistics import median
import json

import names


class Continue(Exception):
    pass


# WORLDGEN CONSTANTS

WORLD_SIZE = [80, 60]  # How big the world is
GEN_RADIUS = 5  # How far away a region can spread in a single worldgen step
SIZE_PARAMETER = 0.2  # Chance for a tile without adjacent terrain to skip this iteration
COASTAL_CITY_DENSITY = 0.15  # Chance for a coastal tile to generate a city
INLAND_CITY_DENSITY = 0.02  # Chance for a landlocked tile to generate a city
MOUNTAIN_CITY_DENSITY = 0.3  # City density multiplier for tiles in difficult terrains
MAX_PRODUCTION_CONSTANT = 1000  # MPC * production value = maximum man-hours that can be spent in a day
POPULATION_GROWTH_CONSTANT = 0.00001  # Multiplier to the rate of population growth
VALUE_CONSTANT = 1.3  # Base which is raised to the power based on supply & demand
TRADE_THRESHOLD = 1  # Minimum units Profit required for a trade to go through
TRADE_VOLUME = 50  # Size of each trade
TRADE_QUANTITY = 30  # Number of trades per year
PREGEN_LENGTH = 0  # Number of 500 years generated before map shows
MAGIC_CONSUMPTION = 0.1  # Percent of magic material consumed versus demanded
NOTABLE_NPC_THRESHOLD = 6  # Number of life events required for an NPC to be remembered by history
# TODO: Politics, Religion

WorldMap = {}
RegionMap = {}
RegionList = []
CityList = []
trade_connections = {}
current_year = 0
Magic = {"Material": ["", (), ""]}  # 0: Name, 1: (rarity, vein size, value), 2: Type (Metal/Gem/Plant)

# (rarity, vein size, value)
Metals = {}
Gems = {}
Plants = {}
# (rarity, vein size, value, taming difficulty)
Animals = {}

Goods = {}
AllItems = []
Biomes = {}

splash_text = ["Religion is not implemented yet", "Bears can be tamed", "Monsters don't actually do anything",
               "NPCs who aren't important are forgotten", "Each region has a unique set of resources",
               "Trade flows when economies are unbalanced", "Not every resource is always available"]


class Inventory:
    def __init__(self, items=None):
        if items is None:
            items = {}
        self.items = items

    def keys(self):
        return self.items.keys()

    def values(self):
        return self.items.values()

    def pop(self, k):
        if k in self.items.keys():
            v = self.items[k]
            del self.items[k]
            return v
        else:
            return 0

    def total(self):
        return sum(self.items.values())

    def normalize(self):
        t = self.total()
        for k in self.items.keys():
            self.items[k] /= t

    def copy(self):
        return Inventory(self.items.copy())

    def __getitem__(self, k):
        if k in self.items.keys():
            return self.items[k]
        else:
            return 0

    def __setitem__(self, k, value):
        if value <= 0 and value in self.items.keys():
            del self.items[k]
        elif value != 0:
            self.items[k] = value

    def __delitem__(self, k):
        del self.items[k]

    def __str__(self):
        return str(self.items)

    def __mul__(self, other):
        inv = Inventory(self.items.copy())
        if isinstance(other, int):
            for i in inv.keys():
                inv[i] *= other
        if isinstance(other, Inventory):
            for i in other.keys():
                inv[i] *= other[i]
        return inv

    def __truediv__(self, other):
        inv = Inventory(self.items.copy())
        if isinstance(other, int):
            for i in inv.keys():
                inv.items[i] /= other
        if isinstance(other, Inventory):
            for i in other.keys():
                if other[i] == 0:
                    inv[i] = 0
                else:
                    inv[i] /= other[i]
        return inv

    def __floor__(self):
        inv = Inventory(self.items.copy())
        for i in inv.keys():
            inv[i] = math.floor(inv[i])
        return inv


class Region:
    def __init__(self, terrain=None):
        self.tiles = []
        self.resources = Inventory()
        if terrain is not None:
            self.terrain = terrain
        else:
            self.terrain = random.choice([*(k for k in Biomes.keys() if k != "Ocean"), "Sea", "Sea"])
        self.ancestor_race = None
        self.adjacent_regions = []
        self.demographics = {}
        for i in range(1):
            self.resources = Inventory(Biomes[self.terrain]["Resources"])
            for r in self.resources.keys():
                self.resources[r] += roundrand() / 10
            if self.resources == {}:
                break
            m = self.resources.pop("Metal")
            for metal in Metals.keys():
                if random.random() > m - Metals[metal][0] / 20:
                    continue
                self.resources[metal] = (roundrand() + 1) * m * Metals[metal][1]
            g = self.resources.pop("Gemstone")
            for gem in Gems.keys():
                if random.random() > g - Gems[gem][0] / 20:
                    continue
                self.resources[gem] = (roundrand() + 1) * g * Gems[gem][1]
            p = self.resources.pop("Plant")
            for plant in Plants.keys():
                if random.random() > p - Plants[plant][0] / 20 or random.random() < 0.8:
                    continue
                self.resources[plant] = (roundrand() + 1) * p * Plants[plant][1]
            a = self.resources.pop("Animal")
            for animal in Animals.keys():
                if random.random() > a - Animals[animal][0] / 20 or random.random() < 0.1:
                    continue
                self.resources[animal] = (roundrand() + 1) * a * Animals[animal][1] / 4
        if Biomes[self.terrain]["Monsters"]:
            self.monster = Monster(self)

    def __str__(self):
        text = self.terrain
        if self.ancestor_race is not None:
            text += " - " + self.ancestor_race
        return text

    def jsonize(self):
        return {"tiles": self.tiles, "ancestor_race": self.ancestor_race, "adjacent_regions": self.adjacent_regions,
                "demographics": self.demographics, "terrain": self.terrain, "resources": self.resources.items,
                "monster": self.monster.jsonize()}

    def desc(self):
        if self.monster:
            return f"This {self.terrain} is home to {self.monster}."
        else:
            return f"This {self.terrain} is free from beasts."


class City:
    def __init__(self, pos):
        self.demographics = RegionList[RegionMap[pos]].demographics
        for d in self.demographics.keys():
            self.demographics[d] += roundrand() / 3
        t = sum([*self.demographics.values()])
        for w in self.demographics.keys():
            self.demographics[w] /= t
        self.region = RegionMap[pos]
        self.majority_race = [*self.demographics.keys()][
            [*self.demographics.values()].index(max(self.demographics.values()))]
        self.name = names.generate(self.majority_race)
        self.pos = pos

        self.npcs = []
        self.data = {}
        self.population = 100
        self.resources = Inventory()
        self.economy = Inventory()
        self.imports = Inventory()
        self.trade = []
        self.production = Inventory()
        self.resource_gathering = (RegionList[self.region].resources * 10).copy()
        for r in self.resource_gathering.keys():
            self.resource_gathering[r] += roundrand() / 10

        self.generate_npc(True, "Ruler")

        self.agriculture = 0
        for k in get_adj(pos):
            if WorldMap[k]["Terrain"] in ["Sea", "Ocean"]:
                self.resource_gathering["Fish"] += 0.1
            for j in get_adj(k):
                if WorldMap[j]["Terrain"] in ["Sea", "Ocean"]:
                    self.agriculture += 0.5
                    break
            else:
                self.agriculture += 0.5

    def generate_npc(self, nobility=False, title="Citizen"):
        race = random.choices([*self.demographics.keys()], [*self.demographics.values()])[0]
        if nobility:
            weights = [*(x ** 5 for x in self.demographics.values())]
            weights = [*(x / sum(weights) for x in weights)]
            race = random.choices([*self.demographics.keys()], weights)[0]
        name = names.generate(race)
        self.npcs.append(NPC(race, name, self.pos, current_year, title))

    def cull_npcs(self):
        self.npcs = [npc for npc in self.npcs if npc.alive or len(npc.life) > NOTABLE_NPC_THRESHOLD]

    def tick(self):
        if self.population == 0:
            return
        self.economy = Inventory()
        demand = Inventory()
        food_resources = Inventory()

        for r in Animals.keys():
            self.resources[f"Tame {r}"] = math.floor(self.resources[f"Tame {r}"] * 0.95)  # Animals die periodically

        for r in self.resource_gathering.keys():
            p = math.floor(min(self.resource_gathering[r] * 1000, self.population * 10)) / 10
            self.resources[r] += p
            self.production[r] += p
            if r in ["Fish", *Plants.keys(), *Animals.keys()]:
                food_resources[r] = self.resources[r]
            if r in [*Metals.keys(), *Gems.keys()]:  # Deplete non-renewable resources
                self.resource_gathering[r] -= p / 1000000
        food_resources.normalize()
        # print(self.resources)
        # print(food_resources)
        # print(food_resources.total())
        for r in AllItems:
            if r in food_resources.keys():
                demand[r] = round(self.population * food_resources[r])
            if demand[r] == 0:
                demand[r] = round(self.population / 1000)
            if r == Magic["Material"][0]:
                demand[r] += round(self.population / 400)
            if r not in self.resource_gathering.keys():
                demand[r] += round(self.population / 1000)
        net_food = 0
        for d in demand.keys():
            # If positive, this is the number of desired but not available. If negative, this is the number extra
            demand[d] -= self.resources[d]
            self.economy[d] = 1
            if d in Goods.keys():
                self.economy[d] = Goods[d][2]
            elif d[:-6] in Metals.keys() and d[-6:] == " Goods":  # Metal goods are 4x as valuable as metal
                self.economy[d] = Metals[d[:-6]][2] * 4
            elif d[4:] in Gems.keys() and d[:4] == "Cut ":  # Cut gems are 10x as valuable as uncut gems
                self.economy[d] = Gems[d[4:]][2] * 10
            elif d[5:] in Animals.keys() and d[:5] == "Tame ":  # Tame animals are 4x as valuable as others
                self.economy[d] = Animals[d[5:]][2] * 4
            exp = demand[d] / (self.population - demand[d] if self.population != demand[d] else 1)
            self.economy[d] *= VALUE_CONSTANT ** exp
            if d not in [*Plants.keys(), Magic["Material"][0], *Animals.keys(), "Fish"]:
                continue
            if d == Magic["Material"][0]:
                if self.resources[d] - MAGIC_CONSUMPTION * (demand[d] + self.resources[d]) < 0:
                    self.resources[d] = 0
                else:
                    self.resources[d] -= MAGIC_CONSUMPTION * (demand[d] + self.resources[d])
                continue
            net_food -= demand[d]
            if demand[d] < 0:
                self.resources[d] = -1 * demand[d]
            else:
                self.resources[d] = 0
        self.economy.normalize()
        self.economy *= len(self.economy.items)
        self.population += math.floor(net_food * POPULATION_GROWTH_CONSTANT)
        if random.random() < net_food - math.floor(net_food * POPULATION_GROWTH_CONSTANT):
            self.population += 1
        # print(self)
        # print(self.resource_gathering)
        # print(self.resources)
        # print(demand)
        # print(self.economy)
        alive_count = 0
        alive_ruler = False
        for npc in self.npcs:
            if npc.alive:
                alive_count += 1
                npc.tick()
                alive_ruler = alive_ruler | (npc.title == "Ruler")
        if not alive_ruler:
            alive_npcs = [npc for npc in self.npcs if npc.alive]
            if alive_npcs:
                new_ruler = random.choice(alive_npcs)
                if new_ruler.race == self.majority_race or random.random() < 0.05 * new_ruler.skills["Leadership"]:
                    new_ruler.title = "Ruler"
                    new_ruler.life.append(LifeEvent(new_ruler, current_year, f"became the ruler of {self.name}"))
        if alive_count < 4 and random.random() < 0.1:
            self.generate_npc()
        if current_year % 100 == 0:
            self.data[str(current_year)] = {"population": self.population, "production": self.production.items.copy(),
                                            "imports": self.imports.items.copy()}
            self.production = Inventory()
            self.imports = Inventory()
            self.cull_npcs()

    def __str__(self):
        return self.name + " (" + self.majority_race + " city), in a " + RegionList[self.region].terrain.lower() + \
               ". Population " + str(self.population)

    def describe(self):
        for npc in self.npcs:
            if not npc.alive:
                continue
            if npc.title == "Ruler":
                ruler = npc
                break
        else:
            return f"This is a {self.majority_race} city in a " \
                   f"{RegionList[self.region].terrain.lower()}. Its population is {self.population}."
        return f"This is a {self.majority_race} city in a " \
               f"{RegionList[self.region].terrain.lower()}. Its population is {self.population}. It is ruled by " \
               f"{ruler}."

    def describe_npcs(self):
        self.cull_npcs()
        return "\n".join((f"<h4>{npc}</h4>{npc.name} was born in Y{npc.birth}. " + " ".join(str(e) for e in npc.life))
                         for npc in self.npcs)

    def tradelog(self):
        k = "<br>".join([*(f"In {t['year']}, {WorldMap[tuple(t['to'])]['Structure'].name} sold {t['get']} for {t['give']}" for t in self.trade)])
        return k

    def jsonize(self):
        self.cull_npcs()

        keys_prod = list(dict.fromkeys([*(vv for k, v in self.data.items() for vv in v["production"])]))
        keys_import = list(dict.fromkeys([*(vv for k, v in self.data.items() for vv in v["imports"])]))
        self.data = {k: ({"population": v["population"], "production": {
            kk: (v["production"][kk] if kk in v["production"].keys() else 0) for kk in keys_prod
        }, "imports": {
            kk: (v["imports"][kk] if kk in v["imports"].keys() else 0) for kk in keys_import
        }}) for k, v in self.data.items()}

        jsondict = {"pos": self.pos, "name": self.name, "population": self.population,
                    "NPCs": [npc.jsonize() for npc in self.npcs], "imports": self.imports.items,
                    "data": self.data, "trade": self.trade, "production": self.production.items,
                    "resources": self.resources.items, "economy": self.economy.items,
                    "resource_gathering": self.resource_gathering.items}
        return jsondict


class NPC:
    def __init__(self, race, name, pos, birth, title="Citizen"):
        self.race = race
        self.lifespan = {"Human": 70, "Dwarf": 120, "Elf": 1000, "Goblin": 25, "Orc": 50}[race]
        self.name = name
        self.title = title
        self.pos = pos
        self.age = 0
        self.inventory = Inventory()
        self.skills = Inventory()
        self.alive = True
        self.birth = birth
        self.life = []
        self.idol = Inventory()
        self.pronoun = random.choice([["he", "him", "his"], ["she", "her", "her"], ["they", "them", "their"]])

    def tick(self):
        if not self.alive:
            return
        self.age += 1
        if (self.age - self.lifespan) / self.lifespan > random.random():
            self.life.append(DeathEvent(self, current_year, "old age"))
            self.alive = False
            return

        # Learning / Studying
        if self.age > 15:
            study_choices = {k: 0.1 + (self.skills[k]) % 12
                             for k in ["Leadership", "Metalworking", "Teaching", "Gemcutting", "Magic",
                                       "Animal Training"]}
            t = sum(study_choices.values())
            study_choices = {k: v / t for k, v in study_choices.items()}
            study = random.choices([*study_choices.keys()], [*study_choices.values()])[0]
            if self.idol[study] == 0:
                self.idol[study] = max(
                    [*(i.skills["Teaching"] * i.skills[study] * (1 if i.alive else 0.5)  # TODO: This feels clunky
                       for i in WorldMap[self.pos]['Structure'].npcs if i != self), 0])
            difficulty = (self.age ** 2) * self.skills[study] / 100 / \
                         max(self.idol[study] - self.skills[study], 10)
            luck = random.random()
            if luck / (1 - luck) > difficulty:  # This can be anywhere from 0 to infinity
                self.skills[study] += 1
                if self.skills[study] == 2:
                    self.life.append(LifeEvent(self, current_year, f"began studying {study.lower()}"))
                elif self.skills[study] == 10:
                    self.life.append(LifeEvent(self, current_year, f"became a master in {study.lower()}"))
                elif self.skills[study] == 20:
                    self.life.append(LifeEvent(self, current_year, f"became an expert in {study.lower()}"))

            # Using productive skills
            metalwork_prod = self.skills["Metalworking"] * 100  # Based on population stuff, a single person shouldn't
            # be able to make this much, but I'm saying this number includes their students and coworkers
            if metalwork_prod > 0:
                metalwork_res = shuffle_dict({k: v for k, v in WorldMap[self.pos]['Structure'].resources.items.items()
                                              if k in Metals.keys()})
                for k, v in metalwork_res.items():  # Metalworking
                    current_goods = min(metalwork_prod, math.floor(v))
                    metalwork_prod -= current_goods
                    WorldMap[self.pos]['Structure'].resources[k] -= current_goods
                    WorldMap[self.pos]['Structure'].resources[k + " Goods"] += current_goods
                    WorldMap[self.pos]['Structure'].production[k + " Goods"] += current_goods
                    if metalwork_prod <= 0:
                        break

            gemwork_prod = self.skills["Gemcutting"] * 100
            if gemwork_prod > 0:
                gemwork_res = shuffle_dict({k: v for k, v in WorldMap[self.pos]['Structure'].resources.items.items()
                                            if k in Gems.keys()})
                for k, v in gemwork_res.items():  # Gemcutting
                    current_goods = min(gemwork_prod, math.floor(v))
                    gemwork_prod -= current_goods
                    WorldMap[self.pos]['Structure'].resources[k] -= current_goods
                    WorldMap[self.pos]['Structure'].resources["Cut " + k] += current_goods
                    WorldMap[self.pos]['Structure'].production["Cut " + k] += current_goods
                    if gemwork_prod <= 0:
                        break

            animal_prod = self.skills["Animal Training"] * 100
            if animal_prod > 0:
                animal_res = shuffle_dict({k: v for k, v in WorldMap[self.pos]['Structure'].resources.items.items()
                                           if k in Animals.keys()})
                for k, v in animal_res.items():  # Animal Training
                    current_goods = min(animal_prod, math.floor(v) / Animals[k][3])
                    animal_prod -= current_goods
                    WorldMap[self.pos]['Structure'].resources[k] -= current_goods
                    WorldMap[self.pos]['Structure'].resources["Tame " + k] += current_goods
                    WorldMap[self.pos]['Structure'].production["Tame " + k] += current_goods
                    if animal_prod <= 0:
                        break

    def __str__(self):
        return f"{self.name}, {self.race} {self.title} of {WorldMap[self.pos]['Structure'].name}"

    def jsonize(self):
        return {"name": self.name, "race": self.race, "title": self.title, "origin": self.pos, "birth": self.birth,
                "alive": self.alive, "inventory": self.inventory.items, "skills": self.skills.items,
                "life": [*(e.jsonize() for e in self.life)]}


class LifeEvent:
    def __init__(self, npc, time, desc=""):
        self.time = time
        self.desc = desc
        self.npc = npc

    def __str__(self):
        return f"In Y{self.time}, {self.npc.pronoun[0]} {self.desc}."

    def jsonize(self):
        return {"Type": "Event", "Time": self.time, "Desc": self.desc}


class DeathEvent(LifeEvent):
    def __str__(self):
        return f"In Y{self.time}, {self.npc.name} died at age {self.time - self.npc.birth} because of {self.desc}."

    def jsonize(self):
        return {"Type": "Death", "Time": self.time, "Desc": self.desc}


class Monster:
    def __init__(self, region):
        if region is None:
            return
        self.species = random.choice(Biomes[region.terrain]["Monsters"])
        self.name = names.generate("Monster", max_length=15)
        mon_color = random.choice(['red', 'blue', 'green', 'black', 'white',
                                   *(k.lower() for k in region.resources.keys() if
                                     k in [*Gems.keys(), *Metals.keys()])])
        if self.species == "Dragon":
            self.desc = f"a great winged reptile with {random.choice(['engraved', 'long', 'sharpened', 'serrated'])} " \
                        f"horns and claws and {mon_color} scales. It " + \
                        random.choice(['is engraved with symbols', 'has a prominent scar', 'wears bone jewelry'])
        elif self.species == "Worm":
            self.desc = f"an enormous worm with {mon_color} plating " + \
                        random.choice(
                            [' engraved with symbols', ' covered in spikes', 'and a fleshy sail along its spine'])
        elif self.species == "Leviathan":
            self.desc = f"a giant sea creature with {random.randint(3, 8) * 2} tentacles, a " \
                        f"{random.choice(['chitinous beak', 'toothy maw'])}, " \
                        f"and {random.choice(['slimy', 'smooth', 'rough', 'bumpy'])}, " \
                        f"{random.choice(['red', 'black', 'gray', 'blue', 'green'])} skin"
        else:
            animals = ['bear', 'beaver', 'gorilla', 'coyote', 'wolf', 'bird', 'deer', 'owl', 'lizard', 'moose',
                       'spider', 'insect', 'lion']
            animal1 = random.choice(animals)
            animal2 = random.choice(animals)
            animal3 = random.choice(['bird', 'bat', 'snake', 'deer', 'moose', 'scorpion', 'elephant'])
            part = {'bird': 'wings', 'bat': 'wings', 'snake': 'fangs', 'deer': 'antlers', 'moose': 'antlers',
                    'spider': 'legs', 'scorpion': 'stinger', 'elephant': 'tusks'}[animal3]
            self.desc = f"an oversized {animal1} " \
                        f"{f'with the head of a {animal2} and ' if animal1 != animal2 else 'with '}the {part} of a {animal3}"

    def __str__(self):
        return f"{self.name}, {self.desc}"

    def jsonize(self):
        return {"name": self.name, "species": self.species, "desc": self.desc}


def roundrand():
    return int(random.random() * 1000) / 1000


def shuffle_dict(_i):
    keys = [i for i in _i.keys()]
    random.shuffle(keys)
    return {k: _i[k] for k in keys}


def blit_txt(txt, x, y, display, text_color=(255, 255, 255), maxlength=0):
    if "\n" in txt:
        text = txt.split("\n")
        y_ = 0
        for n in range(len(text)):
            y_ += blit_txt(text[n], x, y + y_ * 20, display, text_color=text_color, maxlength=maxlength)
        return y_
    if maxlength == 0:
        render = font.render(txt, False, (*text_color, 255))
        display.blit(render, (x, y))
    else:
        text = [txt]
        end = ""
        while len(text[-1]) > maxlength:
            split = text[-1].split(" ")
            last_word = split.pop(-1)
            end = last_word + " " + end
            text[-1] = " ".join(split)
            if len(text[-1]) <= maxlength:
                text.append(end)
                end = ""
        for n in range(len(text)):
            blit_txt(text[n], x, y + 20 * n, display, text_color)
        return len(text)


def update_screen():
    global current_year
    for event in pygame.event.get():
        if event.type == QUIT:
            export_report()
            pygame.quit()
            sys.exit()

    Display.fill((0, 0, 0))

    mouse_pos = pygame.mouse.get_pos()
    m_x = math.floor((mouse_pos[0] - 20) / 10)
    m_y = math.floor((mouse_pos[1] - 20) / 10)

    for k in WorldMap.keys():
        x = k[0] * 10 + 20
        y = k[1] * 10 + 20
        pygame.draw.rect(Display, Biomes[WorldMap[k]["Terrain"]]["Color"], (x, y, 10, 10))

    trade_values = [1]
    for k in trade_connections.keys():
        trade_values.append(trade_connections[k])
    trade_average = sum(trade_values) / len(trade_values)
    for t in trade_connections.keys():
        c1 = (t[0][0] * 10 + 25, t[0][1] * 10 + 25)
        c2 = (t[1][0] * 10 + 25, t[1][1] * 10 + 25)
        pygame.draw.line(Display, (255, 255, 255), c1, c2, min(math.ceil(trade_connections[t] / trade_average), 5))

    population_values = [1]
    for k in CityList:
        WorldMap[k]["Structure"].tick()
        population_values.append(WorldMap[k]["Structure"].population)
    population_average = sum(population_values) / len(population_values)

    for k in CityList:
        x = k[0] * 10 + 25
        y = k[1] * 10 + 25
        population = 0 if population_average == 0 else (WorldMap[k]["Structure"].population / population_average)
        if population == 0:
            pygame.draw.circle(Display, (128, 0, 0), (x, y), 3, 1)
        elif population < 0.5:
            pygame.draw.circle(Display, (0, 0, 0), (x, y), 3, 1)
        elif population < 1.5:
            pygame.draw.circle(Display, (0, 0, 0), (x, y), 4)
        else:
            pygame.draw.circle(Display, (0, 0, 0), (x, y), 5, 2)
        if k == (m_x, m_y):
            blit_txt(str(WorldMap[k]["Structure"]), 20, 625, Display, maxlength=65)

    RegionDisplay = pygame.Surface((900, 700), flags=pygame.SRCALPHA)
    selected_region = None

    for r in RegionList[1:]:
        if (m_x, m_y) in r.tiles:
            selected_region = r
            break

    if selected_region is not None:
        for k in selected_region.tiles:
            x = 10 * k[0] + 20
            y = 10 * k[1] + 20
            pygame.draw.rect(RegionDisplay, (120, 120, 150, 150), (x, y, 10, 10))
        Display.blit(RegionDisplay, (0, 0))

    # if mouse_pos[0] >= 800:
    #    RaceDisplay = pygame.Surface((900, 700), flags=pygame.SRCALPHA)
    #    for r in RegionList[1:]:
    #        if r.ancestor_race is None:
    #            continue
    #        race_color = (0, 0, 0)
    #        if r.ancestor_race == "Human":
    #            race_color = (120, 120, 150)
    #        if r.ancestor_race == "Orc":
    #            race_color = (60, 60, 75)
    #        elif r.ancestor_race == "Elf":
    #            race_color = (120, 150, 120)
    #        elif r.ancestor_race == "Goblin":
    #            race_color = (60, 75, 60)
    #        elif r.ancestor_race == "Dwarf":
    #            race_color = (150, 120, 120)
    #        for k in r.tiles:
    #            x = 10 * k[0] + 20
    #            y = 10 * k[1] + 20
    #            pygame.draw.rect(RaceDisplay, race_color, (x, y, 10, 10))
    #    Display.blit(RaceDisplay, (0, 0))

    pygame.display.update()


def get_adj(pos, ignorebounds=False, r=1):
    adj = []
    if r == 0:
        for a in [(0, 1), (1, 0), (0, -1), (-1, 0)]:
            adj.append((a[0] + pos[0], a[1] + pos[1]))
        return adj
    for x in range(2 * r + 1):
        for y in range(2 * r + 1):
            if (x, y) == (r, r):
                continue
            if ignorebounds or (pos[0] + x - r, pos[1] + y - r) in WorldMap.keys():
                adj.append((pos[0] + x - r, pos[1] + y - r))
    return adj


def manage_trade(route):
    c1 = WorldMap[route[0]]["Structure"]
    c2 = WorldMap[route[1]]["Structure"]
    c1_demand = Inventory()  # c1 trade => gift to c1
    c2_demand = Inventory()  # c2 trade => gift to c2
    for k in c1.economy.keys():
        if c1.economy[k] <= 0:
            continue
        c1_demand[k] = TRADE_VOLUME / c1.economy[k]
    for k in c2.economy.keys():
        if c2.economy[k] <= 0:
            continue
        c2_demand[k] = TRADE_VOLUME / c2.economy[k]
    c1_supply = c1_demand * c2.economy
    c2_supply = c2_demand * c1.economy
    # print("Demand")
    # print(c1_demand)
    # print(c2_demand)
    # print("Economy")
    # print(c1.economy)
    # print(c2.economy)
    # print("Supply")
    # print(c1_supply)
    # print(c2_supply)
    # print("Resources")
    # print(c1.resources)
    # print(c2.resources)
    if len(c1_supply.items) == 0 or len(c2_supply.items) == 0:
        return
    try:
        c1_min = min(c1_supply.values())
        c1_trade = \
            [(key, value, math.floor(c1_demand[key])) for key, value in c1_supply.items.items() if value == c1_min][0]
        c2_min = min(c2_supply.values())
        c2_trade = \
            [(key, value, math.floor(c2_demand[key])) for key, value in c2_supply.items.items() if value == c2_min][0]
        if c1_trade[1] > (TRADE_VOLUME - TRADE_THRESHOLD) or c2_trade[1] > (TRADE_VOLUME - TRADE_THRESHOLD) or \
                c1.resources[c2_trade[0]] < c2_trade[2] or c2.resources[c1_trade[0]] < c1_trade[2]:
            return
        c1.resources[c1_trade[0]] += c1_trade[2]
        c1.imports[c1_trade[0]] += c1_trade[2]
        c2.resources[c1_trade[0]] -= c1_trade[2]
        c2.imports[c1_trade[0]] -= c1_trade[2]

        c1.resources[c2_trade[0]] -= c2_trade[2]
        c1.imports[c2_trade[0]] -= c2_trade[2]
        c2.resources[c2_trade[0]] += c2_trade[2]
        c2.imports[c2_trade[0]] += c2_trade[2]

        c1.trade.append({"to": route[1], "give": f"{c2_trade[2]} {c2_trade[0]}", "get": f"{c1_trade[2]} {c1_trade[0]}",
                         "year": current_year})
        c2.trade.append({"to": route[0], "give": f"{c1_trade[2]} {c1_trade[0]}", "get": f"{c2_trade[2]} {c2_trade[0]}",
                         "year": current_year})

        trade_connections[route] += 1
        # print(f"{c1.name} and {c2.name} Trade {c1_trade[2]} {c1_trade[0]} for {c2_trade[2]} {c2_trade[0]}.")
    except IndexError:
        return


def generate_map():
    global RegionList, RegionMap
    Regions = 0
    for x in range(WORLD_SIZE[0]):  # Set up coords
        for y in range(WORLD_SIZE[1]):
            WorldMap[(x, y)] = None
            if x in [0, WORLD_SIZE[0] - 1] or y in [0, WORLD_SIZE[1] - 1]:
                RegionMap[(x, y)] = 0
            else:
                RegionMap[(x, y)] = None

    keys = [*RegionMap.keys()]

    while True:
        random.shuffle(keys)
        for k in keys:
            if RegionMap[k] is not None:
                continue
            for n in range(GEN_RADIUS):  # Look for adjacent tiles before starting a new Region
                adj = get_adj(k, r=n)
                adjacent_terrain = []
                for a in adj:
                    if RegionMap[a] is not None:
                        adjacent_terrain.append(RegionMap[a])
                if adjacent_terrain:
                    RegionMap[k] = random.choice(adjacent_terrain)
                    break
            else:  # Start a new Region
                if random.random() < SIZE_PARAMETER:
                    continue
                Regions += 1
                RegionMap[k] = Regions
        for k in keys:
            if RegionMap[k] is None:
                break
        else:
            break

    RegionList = [Region(terrain="Ocean")]  # Generate the Regions
    for r in range(Regions):
        RegionList.append(Region())

    for k in RegionMap.keys():  # Apply the Regions' terrains
        WorldMap[k] = {"Terrain": RegionList[RegionMap[k]].terrain}
        RegionList[RegionMap[k]].tiles.append(k)

    for k in RegionMap.keys():  # Find adjacent Regions
        adj = get_adj(k)
        for a in adj:
            if RegionMap[a] not in RegionList[RegionMap[k]].adjacent_regions and RegionMap[a] != RegionMap[k]:
                RegionList[RegionMap[k]].adjacent_regions.append(RegionMap[a])


def generate_cities():
    keys = WorldMap.keys()
    possible_cities = []
    for k in keys:
        if WorldMap[k]["Terrain"] in ["Ocean", "Sea"]:
            continue
        adj = get_adj(k)
        adj_terrain = [*(WorldMap[a]["Terrain"] for a in adj)]
        if "Ocean" in adj_terrain or "Sea" in adj_terrain:
            if random.random() < COASTAL_CITY_DENSITY * (
                    MOUNTAIN_CITY_DENSITY if WorldMap[k]["Terrain"] in ["Mountain", "Desert"] else 1):
                possible_cities.append(k)
        else:
            if random.random() < INLAND_CITY_DENSITY * (
                    MOUNTAIN_CITY_DENSITY if WorldMap[k]["Terrain"] in ["Mountain", "Desert"] else 1):
                possible_cities.append(k)
    random.shuffle(possible_cities)
    for p in possible_cities:
        adj = get_adj(p)
        for a in adj:
            if a in CityList:
                break
        else:
            CityList.append(p)
    for c in CityList:
        WorldMap[c]["Structure"] = City(c)
    possible_trade_connections = {}
    for i in range(len(CityList)):
        c1 = CityList[i]
        d1 = 2 if WorldMap[c1]["Terrain"] == "Mountain" else 1.4 if WorldMap[c1]["Terrain"] == "Desert" else \
            1.2 if WorldMap[c1]["Terrain"] == "Forest" else 1
        for ii in range(len(CityList)):
            if i >= ii:
                continue
            c2 = CityList[ii]
            d2 = 2 if WorldMap[c2]["Terrain"] == "Mountain" else 1.4 if WorldMap[c2]["Terrain"] == "Desert" else \
                1.2 if WorldMap[c2]["Terrain"] == "Forest" else 1
            d = math.sqrt((c1[0] - c2[0]) ** 2 + (c1[1] - c2[1]) ** 2) * d1 * d2
            possible_trade_connections[(c1, c2)] = d
    for c in CityList:
        connections = {}
        for k in possible_trade_connections.keys():
            if c not in k:
                continue
            connections[k] = possible_trade_connections[k]
        connections = {k: v for k, v in sorted(connections.items(), key=lambda item: item[1])}
        for n in range(min(3, len(connections))):
            t = [*connections.keys()][n]
            if t not in trade_connections.keys():
                trade_connections[t] = 0


def generate_magic():
    Magic['Localization'] = "Ubiquitous"
    r_i = random.randint(0, 4)
    # rarity = ["extremely rare", "very rare", "rare", "common", "very common"][r_i]
    if random.random() < 0.6:
        Magic['Localization'] = "Localized"
    Magic["Material"][1] = ()
    Magic["Material"][2] = random.choice(["Metal", "Plant", "Gemstone"])
    Magic["Material"][0] = names.generate(Magic["Material"][2])
    if Magic["Material"][2] in ["Metal", "Gemstone"]:
        Magic["Material"][1] = (1 if Magic['Localization'] == "Ubiquitous" else 6, 2 + r_i, 9)
    elif Magic["Material"][2] == "Plant":
        Magic["Material"][1] = ((4 if Magic['Localization'] == "Ubiquitous" else 10) - r_i, 1, 9)
    yield  # Run the second call after regions are generated
    if Magic['Localization'] == "Localized":  # Get rid of all of the resource other than one random location
        select_regions = [r for r in RegionList if hasattr(r, "resources") and r.resources[Magic["Material"][0]] > 0]
        cl = CityList.copy()
        random.shuffle(cl)
        for c in CityList:
            city = WorldMap[c]["Structure"]
            if city.region in select_regions:
                select_regions.remove(city.region)
        for r in select_regions:
            r.resources[Magic["Material"][0]] = 0
    # TODO: Add the possible magic abilities
    # Increase own life span
    # Generate Resources


def set_races():
    randlist = [*range(len(RegionList))]
    random.shuffle(randlist)
    checklist = {"Human": False, "Elf": False, "Dwarf": False, "Orc": False, "Goblin": False}
    for r in randlist:  # Seed original three
        if RegionList[r].terrain in ["Plain"]:
            if not checklist["Human"]:
                RegionList[r].ancestor_race = "Human"
                checklist["Human"] = True
            elif not checklist["Orc"]:
                RegionList[r].ancestor_race = "Orc"
                checklist["Orc"] = True
        elif RegionList[r].terrain in ["Forest"]:
            if not checklist["Elf"]:
                RegionList[r].ancestor_race = "Elf"
                checklist["Elf"] = True
            elif not checklist["Goblin"]:
                RegionList[r].ancestor_race = "Goblin"
                checklist["Goblin"] = True
        elif RegionList[r].terrain in ["Mountain"]:
            if not checklist["Dwarf"]:
                RegionList[r].ancestor_race = "Dwarf"
                checklist["Dwarf"] = True
        for k in checklist.values():
            if not k:
                break
        else:
            break

    for n in range(5):  # Spread into adjacent regions
        random.shuffle(randlist)
        for r in randlist:
            if RegionList[r].ancestor_race is None:
                continue
            if RegionList[r].ancestor_race in ["Orc", "Dwarf", "Goblin"] and random.random() < 0.6:
                continue
            if RegionList[r].ancestor_race not in ["Human"] and random.random() < 0.3:
                continue
            adjacent = RegionList[r].adjacent_regions
            for k in RegionList[r].adjacent_regions:
                if RegionList[k].terrain in ["Ocean", "Sea"] or RegionList[k].ancestor_race is not None:
                    adjacent.remove(k)
            if adjacent:
                adj = random.choice(adjacent)
                RegionList[adj].ancestor_race = RegionList[r].ancestor_race

    for r in RegionList:  # Find race makeup of different regions
        if r.terrain in ["Sea", "Ocean"]:
            continue
        weights = {"Human": 2.0 + roundrand(), "Elf": 1.0 + roundrand(), "Dwarf": 1.0 + roundrand(),
                   "Orc": 0.3 + roundrand(), "Goblin": 0.2 + roundrand()}
        if r.ancestor_race is not None:
            weights[r.ancestor_race] += 5.0
        for k in r.adjacent_regions:
            if RegionList[k].ancestor_race is not None:
                weights[RegionList[k].ancestor_race] += 1.0
        if r.terrain == "Forest":
            weights["Elf"] += 0.5
        elif r.terrain == "Mountain":
            weights["Dwarf"] += 1.0
        for w in weights.keys():
            weights[w] /= {"Human": 70, "Dwarf": 120, "Elf": 1000, "Goblin": 25, "Orc": 50}[w]
        total = sum(weights.values())
        for w in weights.keys():
            weights[w] /= total
        r.demographics = weights


def export_map(start, end, selection=None, scale=1.0, trade=False):
    if selection is None:
        selection = []
    environment = Environment(loader=FileSystemLoader("templates/"))
    results_template = environment.get_template("map.html")
    context = {
        "WorldMap": WorldMap,
        "CityList": CityList,
        "ResourceLink": resource_link,
        "Range": ((median([0, start[0], WORLD_SIZE[0]]), median([0, start[1], WORLD_SIZE[1]])),
                  (median([0, end[0], WORLD_SIZE[0]]), median([0, end[1], WORLD_SIZE[1]]))),
        "Selection": selection,
        "Scale": scale,  # TODO: This might not generate the right dictionary
        "Colors": {k: "#" + hex(sum(i * (256 ** (2 - ii)) for ii, i in enumerate(v["Color"])))[2:] for k, v in Biomes.items()},
        "TradeRoutes": [*(k for k in trade_connections.keys() if trade_connections[k] > 1)] if trade else [],
        "min": min,
        "abs": abs
    }
    return results_template.render(context)


def export_report():
    # print(json.dumps(jsonize()))
    environment = Environment(loader=FileSystemLoader("templates/"))
    results_filename = f"reports/{Magic['Material'][0]}_{current_year}.html"

    with open(f"json_data/{Magic['Material'][0]}_{current_year}.json", "w") as json_dump:
        json_dump.write(json.dumps(jsonize()))

    results_template = environment.get_template("report.html")
    ResourceProduction = {}
    for k in CityList:
        for r in WorldMap[k]["Structure"].resource_gathering.keys():
            if r in ResourceProduction.keys():
                ResourceProduction[r].append(k)
            else:
                ResourceProduction[r] = [k]
    del ResourceProduction['Fish']

    def resource_segment(res):
        res_type = "resource"
        if res in Metals.keys():
            res_type = "metal"
        elif res in Plants.keys():
            res_type = "plant"
        elif res in Gems.keys():
            res_type = "gem"
        elif res in Animals.keys():
            res_type = "animal"
        if res in ResourceProduction.keys():
            return f"<h3 id='res_{res}'>{res}</h3>This {res_type} is found in the following locations:" \
                   f"{export_map((min(i[0] for i in ResourceProduction[res]) - 4, min(i[1] for i in ResourceProduction[res]) - 4), (max(i[0] for i in ResourceProduction[res]) + 5, max(i[1] for i in ResourceProduction[res]) + 5), scale=0.5, selection=ResourceProduction[res])}"
        return ""

    def region_segment(reg):
        return f"<h3 id='reg_{reg}'>{reg}</h3>{reg.desc()}" \
               f"{export_map((min(i[0] for i in reg.tiles) - 4, min(i[1] for i in reg.tiles) - 4), (max(i[0] for i in reg.tiles) + 4, max(i[1] for i in reg.tiles) + 4), scale=0.5, selection=reg.tiles)}"

    context = {
        "WorldMap": WorldMap,
        "CityList": CityList,
        "ResourceLink": resource_link,
        "ExportMap": export_map,
        "ResourceSegment": resource_segment,
        "RegionSegment": region_segment,
        "Resources": AllItems,
        "Regions": RegionList[1:],
        "Magic": Magic,
        "WORLD_SIZE": WORLD_SIZE
    }
    with open(results_filename, mode="w") as results:
        results.write(results_template.render(context))
        print(f"... wrote {results_filename}")


def resource_link(res):
    return f"<a href=#res_{res}>{res}</a>"


def jsonize():
    return {"file_type": "save",
            "RegionList": [r.jsonize() for r in RegionList[1:]],
            "CityList": [WorldMap[k]["Structure"].jsonize() for k in CityList],
            "trade_connections": {str([k[0][0], k[0][1], k[1][0], k[1][1]]): v for k, v in trade_connections.items()},
            "Biomes": Biomes,
            "Items": {
                "Animals": Animals,
                "Gems": Gems,
                "Metals": Metals,
                "Plants": Plants},
            "current_year": current_year,
            "Magic": Magic,
            "Config": {"GEN_RADIUS": GEN_RADIUS, "SIZE_PARAMETER": SIZE_PARAMETER,
                       "COASTAL_CITY_DENSITY": COASTAL_CITY_DENSITY, "INLAND_CITY_DENSITY": INLAND_CITY_DENSITY,
                       "MOUNTAIN_CITY_DENSITY": MOUNTAIN_CITY_DENSITY,
                       "MAX_PRODUCTION_CONSTANT": MAX_PRODUCTION_CONSTANT,
                       "POPULATION_GROWTH_CONSTANT": POPULATION_GROWTH_CONSTANT, "TRADE_THRESHOLD": TRADE_THRESHOLD,
                       "TRADE_VOLUME": TRADE_VOLUME, "TRADE_QUANTITY": TRADE_QUANTITY,
                       "PREGEN_LENGTH": PREGEN_LENGTH,
                       "MAGIC_CONSUMPTION": MAGIC_CONSUMPTION,
                       "NOTABLE_NPC_THRESHOLD": NOTABLE_NPC_THRESHOLD}}


pygame.init()
pygame.font.init()
font_size = 20
font = pygame.font.Font('PTM55FT.ttf', font_size)

Display = pygame.display.set_mode((900, 700))
pygame.display.set_caption('Continent')


def simulate():
    global current_year
    while True:
        update_screen()
        for n in range(TRADE_QUANTITY):
            manage_trade(random.choice([*trade_connections.keys()]))
        current_year += 1
        # for key in WorldMap.keys():
        #     if "Structure" not in WorldMap[key].keys():
        #         continue
        #     # WorldMap[key]["Structure"].tick()


def init_magic():
    AllItems.append(Magic["Material"][0])
    if Magic["Material"][2] == "Gemstone":
        Gems[Magic["Material"][0]] = Magic["Material"][1]
    elif Magic["Material"][2] == "Metal":
        Metals[Magic["Material"][0]] = Magic["Material"][1]
    elif Magic["Material"][2] == "Plant":
        Plants[Magic["Material"][0]] = Magic["Material"][1]


def pregen(plen):
    global current_year
    Display.fill((96, 96, 96))
    blit_txt("Simulating...", 250, 250, Display)
    pygame.draw.rect(Display, (0, 0, 0), (250, 320, 500, 40))
    pygame.display.update()
    for n in range(500):
        pygame.draw.rect(Display, (96, 96, 150), (250, 320, n, 40))
        pygame.display.update((250, 320, n, 40))
        if current_year % 100 == 0:
            txt = random.choice(splash_text)
            pygame.draw.rect(Display, (96, 96, 96), (250, 400, 500, 40))
            blit_txt(txt, 250, 400, Display)
            pygame.display.update((250, 400, 500, 40))
        for event in pygame.event.get():
            if event.type == QUIT:
                pygame.quit()
                sys.exit()
        for m in range(plen):
            for key in CityList:
                WorldMap[key]["Structure"].tick()
            for t in range(TRADE_QUANTITY):
                manage_trade(random.choice([*trade_connections.keys()]))
            current_year += 1


def init_world():
    global current_year
    gm = generate_magic()
    init_magic()
    generate_map()
    next(gm)
    set_races()
    generate_cities()
    pregen(PREGEN_LENGTH)
