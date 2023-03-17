# sim.py is the main code for the game
# sim_template.py is the Jinja template form

import pygame
from pygame.locals import *
from jinja2 import Environment, FileSystemLoader
import sys
import random
import math
from statistics import median
import json

# from multiprocessing import Pool, Process
import names


class Continue(Exception):
    pass


# TODO: Events directly impact culture
# TODO: War - units cost resources
# TODO: Upper and Lower Class - Upper Class has NPCs, Lower Class has laborers

config = {}
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

splash_text = ["Religion is not implemented yet", "Bears can be tamed", "Monsters steal from cities & kill people",
               "NPCs who aren't important are forgotten", "Each region has a unique set of resources",
               "Trade flows when economies are unbalanced", "Not every resource is always available", "Magic is useful",
               "Philosophers can spread their ideas", "HISTORY_EVENT", "HISTORY_EVENT", "HISTORY_EVENT"]


class Region:
    def __init__(self, terrain=None):
        self.tiles = []
        self.resources = {}
        if terrain is not None:
            self.terrain = terrain
        else:
            self.terrain = random.choice([*(k for k in Biomes.keys() if k != "Ocean"), "Sea", "Sea"])
        self.ancestor_race = None
        self.adjacent_regions = []
        self.demographics = {}
        for i in range(1):
            try:
                self.resources = Biomes[self.terrain]["Resources"].copy()
            except KeyError:
                self.resources = {}
            for r in self.resources.keys():
                self.resources[r] += roundrand() / 10
            if self.resources == {}:
                break
            metal_amount = self.resources.pop("Metal")
            for metal in Metals.keys():
                if random.random() > metal_amount - Metals[metal][0] / 20:
                    continue
                self.resources[metal] = (roundrand() + 1) * metal_amount * Metals[metal][1]
            gemstone_amount = self.resources.pop("Gemstone")
            for gem in Gems.keys():
                if random.random() > gemstone_amount - Gems[gem][0] / 20:
                    continue
                self.resources[gem] = (roundrand() + 1) * gemstone_amount * Gems[gem][1]
            plant_amount = self.resources.pop("Plant")
            for plant in Plants.keys():
                if random.random() > plant_amount - Plants[plant][0] / 20 or random.random() < 0.8:
                    continue
                self.resources[plant] = (roundrand() + 1) * plant_amount * Plants[plant][1]
            animal_amount = self.resources.pop("Animal")
            for animal in Animals.keys():
                if random.random() > animal_amount - Animals[animal][0] / 20 or random.random() < 0.1:
                    continue
                self.resources[animal] = (roundrand() + 1) * animal_amount * Animals[animal][1] / 4
        try:
            if Biomes[self.terrain]["Monsters"]:
                self.monster = Monster(self)
        except KeyError:
            pass

    def __str__(self):
        return self.terrain + str(self.tiles)

    def tick(self):
        if self.monster:
            self.monster.tick(self)

    def jsonize(self):
        return {"tiles": self.tiles, "ancestor_race": self.ancestor_race, "adjacent_regions": self.adjacent_regions,
                "demographics": self.demographics, "terrain": self.terrain, "resources": self.resources,
                "monster": self.monster.jsonize()}

    def desc(self):
        if self.monster:
            return f"This {self.terrain} {'is' if self.monster.alive else 'was'} home to {self.monster}."
        else:
            return f"This {self.terrain} is free from beasts."


class City:
    def __init__(self, pos):
        self.demographics = RegionList[RegionMap[pos]].demographics
        for d in self.demographics.keys():
            self.demographics[d] += roundrand() / 3
        total_demographics = sum([*self.demographics.values()])
        for d in self.demographics.keys():
            self.demographics[d] /= total_demographics
        self.region = RegionMap[pos]
        self.majority_race = [*self.demographics.keys()][
            [*self.demographics.values()].index(max(self.demographics.values()))]
        self.name = names.generate(self.majority_race)
        self.pos = pos

        self.npcs = []
        self.armies = []
        self.history = []
        self.data = {}
        self.library = {}
        self.population = 100
        self.homunculi = 0
        self.artifacts = []
        self.resources = {}
        self.economy = {}
        self.imports = {}
        self.trade = []
        self.production = {}
        self.resource_gathering = {k: v * 10 for k, v in RegionList[self.region].resources.items()}
        for r in self.resource_gathering.keys():
            self.resource_gathering[r] += roundrand() / 10

        self.generate_npc(True, "Ruler")

        self.agriculture = 0
        for k in get_adj(pos, bounds=WorldMap.keys()):
            if WorldMap[k]["Terrain"] in ["Sea", "Ocean"]:
                {{inv_opeq('self.resource_gathering', '"Fish"', '+', '0.1')}}
            for j in get_adj(k, bounds=WorldMap.keys()):
                if WorldMap[j]["Terrain"] in ["Sea", "Ocean"]:
                    self.agriculture += 0.5
                    break
            else:
                self.agriculture += 0.5

        self.cultural_values = {}
        for k in ["Individualism", "Formality", "Tradition", "Equality", "Art", "Knowledge", "Might"]:
            self.cultural_values[k] = random.randint(1, 5)

    def generate_npc(self, nobility=False, title="Citizen"):
        race = random.choices([*self.demographics.keys()], [*self.demographics.values()])[0]
        if nobility:
            weights = [*(x ** 5 for x in self.demographics.values())]
            weight_sum = sum(weights)
            weights = [*(x / weight_sum for x in weights)]
            race = random.choices([*self.demographics.keys()], weights)[0]
        name = names.generate(race)
        self.npcs.append(NPC(race, name, self.pos, current_year, title))

    def cull_npcs(self):
        self.npcs = [npc for npc in self.npcs if npc.alive or len(npc.life) > config["NOTABLE_NPC_THRESHOLD"]]

    def tick(self):
        if self.population == 0:
            return

        military_score = 0
        for army in self.armies:
            military_score += army["Size"]
        military_score *= config["ARMY_SIZE"] / self.cultural_values["Might"] / config["ARMY_PARAMETER"]
        if military_score < self.population and self.population > 10 * config["ARMY_SIZE"]:
            self.create_army()
            self.population -= config["ARMY_SIZE"]

        self.economy = {}
        demand = {}
        food_resources = {}

        for item in Animals.keys():
            self.resources[f"Tame {item}"] = math.floor(
                {{getinv('self.resources', 'f"Tame {item}"')}} * 0.9)  # Animals die periodically

        for item in self.resource_gathering.keys():
            assert self.resource_gathering[
                       item] >= 0, f"{self.name} had {self.resource_gathering[item]} {item}. Somethin' ain't right."
            assert self.homunculi >= 0, f"{self.name} had {self.homunculi} homunculi. Somethin' ain't right."
            production = \
                {{resistance_add('self.resource_gathering[item] * config["MAX_PRODUCTION_CONSTANT"]', '(self.population + self.homunculi) * 2')}}
            if production < 0:
                print(production)
            {{inv_opeq('self.resources', 'item', '+', 'production')}}
            {{inv_opeq('self.production', 'item', '+', 'production')}}
            if item in ["Fish", *Plants.keys(), *Animals.keys()]:
                food_resources[item] = {{getinv('self.resources', 'item')}}
            if item in [*Metals.keys(), *Gems.keys()]:  # Deplete non-renewable resources
                self.resource_gathering[item] -= production / 1000000
        total_food_resources = sum(food_resources.values())
        for k in food_resources.keys():
            food_resources[k] /= total_food_resources
        # print(self.resources)
        # print(food_resources)
        # print(food_resources.total())
        for item in AllItems:
            if item in food_resources.keys():
                demand[item] = round(self.population * food_resources[item])
            if {{getinv('demand', 'item')}} == 0:
                demand[item] = round(self.population / 1000)
            if item == Magic["Material"][0]:
                demand[item] += round(self.population / 400)
            if item not in self.resource_gathering.keys():
                demand[item] += round(self.population / 1000)
        net_food = 0
        for demanded_item in demand.keys():
            # If positive, this is the number of desired but not available. If negative, this is the number extra
            demand[demanded_item] -= {{getinv('self.resources', 'demanded_item')}}
            self.economy[demanded_item] = 1
            mat = {{extract_material('demanded_item')}}
            if demanded_item in Goods.keys():
                self.economy[demanded_item] = Goods[demanded_item][2]
            elif mat in Metals.keys() and demanded_item[:-6] == " Goods":  # Metal goods are 4x as valuable as metal
                self.economy[demanded_item] = Metals[mat][2] * 4
            elif mat in Gems.keys() and demanded_item[:4] == "Cut ":  # Cut gems are 10x as valuable as uncut gems
                self.economy[demanded_item] = Gems[mat][2] * 10
            elif mat in Animals.keys() and demanded_item[:5] == "Tame ":  # Tame animals are 4x as valuable as others
                self.economy[demanded_item] = Animals[mat][2] * 4
            exp = demand[demanded_item] / (
                self.population - demand[demanded_item] if self.population != demand[demanded_item] else 1)
            self.economy[demanded_item] *= 1.05 ** exp
            if demanded_item not in [*Plants.keys(), Magic["Material"][0], *Animals.keys(), "Fish"]:
                continue
            if demanded_item == Magic["Material"][0]:
                if {{getinv('self.resources', 'demanded_item')}} - config["MAGIC_CONSUMPTION"] * (
                        demand[demanded_item] + {{getinv('self.resources', 'demanded_item')}}) < 0:
                    self.resources[demanded_item] = 0
                else:
                    {{inv_opeq('self.resources', 'demanded_item', '-',
                               'config["MAGIC_CONSUMPTION"] * (demand[demanded_item] + ' + getinv('self.resources',
                                                                                                  'demanded_item') + ')')}}
                continue
            net_food -= demand[demanded_item]
            net_food -= {{getinv('self.resources', 'demanded_item')}} / 2
            if demand[demanded_item] < 0:
                self.resources[demanded_item] = -1 * demand[demanded_item]
            else:
                self.resources[demanded_item] = 0
        # Normalize Economy
        total_economy = sum(self.economy.values())
        for k in self.economy.keys():
            self.economy[k] /= total_economy
        # Population Growth
        self.population += math.floor(net_food * config["POPULATION_GROWTH_CONSTANT"])
        if random.random() < net_food * config["POPULATION_GROWTH_CONSTANT"] - math.floor(
                net_food * config["POPULATION_GROWTH_CONSTANT"]):
            self.population += 1
        # print(self)
        # print(self.resource_gathering)
        # print(self.resources)
        # print(demand)
        # print(self.economy)
        for army in self.armies:
            if army["Size"] > 0:
                self.tick_army(army)
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
                if new_ruler.race == self.majority_race or random.random() < 0.05 * \
                        {{getinv('new_ruler.skills', '"Leadership"')}} or new_ruler.pos == self.pos:
                    new_ruler.title = "Ruler"
                    new_ruler.reputation += 2
                    new_ruler.life.append(LifeEvent(new_ruler, current_year, f"became the ruler of {self.name}"))
        if alive_count < self.population / config["NPC_COUNT"] and random.random() < 0.1:
            self.generate_npc()
        # Keep track of data
        if current_year % 100 == 0:
            self.data[str(current_year)] = {"population": self.population, "production": self.production.copy(),
                                            "imports": self.imports.copy()}
            self.production = {}
            self.imports = {}
            self.cull_npcs()

    def create_army(self):
        self.armies.append({"Name": f"{self.name} Infantry", "Pos": self.pos, "ATK": 3, "DEF": 12, "POW": 2, "TOU": 12,
                            "MOR": 1, "COM": 2, "Attacks": 1, "Damage": 1, "Size": 6, 'Diminished': False,
                            "Origin": self.pos})

    def tick_army(self, army):
        encounter = []
        for c in CityList:
            other_city = WorldMap[c]["Structure"]
            if other_city.name == self.name:
                continue
            if army["Pos"] == c:
                cultural_differences = {k: abs(v - other_city.cultural_values[k]) for k, v in
                                        self.cultural_values.items()}
                biggest_difference = max(cultural_differences.values())
                if biggest_difference < 2:
                    biggest_difference = 10
                for k, v in self.cultural_values.items():
                    if cultural_differences[k] < biggest_difference:
                        continue
                    other_city.history.append(CityEvent(other_city, current_year,
                                                        f"was convinced to {'reject' if self.cultural_values[k] < 3 else 'accept' if self.cultural_values[k] > 3 else 'become neutral to'} {k}"))
                    return
            for army_n in WorldMap[c]["Structure"].armies:
                if army["Pos"] == army_n["Pos"] and army_n["Size"] > 0 and army_n["Origin"] != self.pos and \
                        {{cultural_distance("self", "other_city")}} > 45:
                    encounter.append(army_n)
        if len(encounter) == 0:
            traveler_options = [
                *(k for k in get_adj(army["Pos"], bounds=WorldMap.keys()) if
                  (abs(k[0] - self.pos[0]) + abs(k[1] - self.pos[1])) < 10)]
            army["Pos"] = random.choice(traveler_options)
            return
        if len(encounter) > 1:
            random.shuffle(encounter)
        other_army = encounter[0]
        attack_total = random.randint(1, 20) + army["ATK"]
        if attack_total < other_army["DEF"]:
            # Failed attack
            return
        other_army["Size"] -= army["Damage"]
        power_total = random.randint(1, 20) + army["POW"]
        if power_total >= other_army["TOU"]:
            other_army["Size"] -= army["Damage"]
            if other_army["Size"] < 3 and not other_army["Diminished"]:
                other_army["Diminished"] = True
                morale_total = random.randint(1, 20) + other_army["MOR"]
                if morale_total < 12:
                    other_army["Size"] -= 1
        if other_army["Size"] <= 0:
            self.history.append(
                CityEvent(self, current_year, f"learned that {army['Name']} defeated {other_army['Name']}"))
            other_city = WorldMap[other_army["Origin"]]["Structure"]
            other_city.history.append(CityEvent(other_city, current_year,
                                                f"learned that {other_army['Name']} was defeated by {army['Name']}"))

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

    def describe_culture(self):
        description = ""
        refactored_values = {k: [*(v for v in self.cultural_values.keys() if self.cultural_values[v] == k)] for k in
                             range(1, 6)}
        if len(refactored_values[1]) != 0:
            addition = {{str_list('refactored_values[1]')}} + (
                " is" if len(refactored_values[1]) == 1 else " are") + " seen as deplorable. "
            description += addition[0].upper() + addition[1:]
        if len(refactored_values[2]) != 0:
            addition = {{str_list('refactored_values[2]')}} + (
                " is" if len(refactored_values[2]) == 1 else " are") + " considered useless. "
            description += addition[0].upper() + addition[1:]
        if len(refactored_values[3]) != 0:
            addition = {{str_list('refactored_values[3]')}} + (
                " is" if len(refactored_values[3]) == 1 else " are") + " viewed neutrally. "
            description += addition[0].upper() + addition[1:]
        if len(refactored_values[4]) != 0:
            addition = {{str_list('refactored_values[4]')}} + (
                " is" if len(refactored_values[4]) == 1 else " are") + " important. "
            description += addition[0].upper() + addition[1:]
        if len(refactored_values[5]) != 0:
            addition = {{str_list('refactored_values[5]')}} + (
                " is" if len(refactored_values[5]) == 1 else " are") + " extremely important. "
            description += addition[0].upper() + addition[1:]
        return description

    def describe_history(self):
        return " ".join(str(e) for e in self.history)

    def tradelog(self):
        return "<br>".join([*(
            f"In {t['year']}, {WorldMap[tuple(t['to'])]['Structure'].name} sold {t['get']} for {t['give']}" for t in
            self.trade)])

    def jsonize(self):
        self.cull_npcs()

        keys_prod = list(dict.fromkeys([*(vv for k, v in self.data.items() for vv in v["production"])]))
        keys_import = list(dict.fromkeys([*(vv for k, v in self.data.items() for vv in v["imports"])]))
        self.data = {k: ({"population": v["population"], "production": {
            kk: (v["production"][kk] if kk in v["production"].keys() else 0) for kk in keys_prod
        }, "imports": {
            kk: (v["imports"][kk] if kk in v["imports"].keys() else 0) for kk in keys_import
        }}) for k, v in self.data.items()}

        jsondict = {"pos": self.pos, "name": self.name, "population": self.population, "homunculi": self.homunculi,
                    "NPCs": [npc.jsonize() for npc in self.npcs], "imports": self.imports, "artifacts": self.artifacts,
                    "data": self.data, "trade": self.trade, "production": self.production,
                    "resources": self.resources, "economy": self.economy, "library": self.library,
                    "resource_gathering": self.resource_gathering, "cultural_values": self.cultural_values,
                    "history": [*(e.jsonize() for e in self.history)]}
        return jsondict


class NPC:
    def __init__(self, race, name, pos, birth, title="Citizen"):
        self.race = race
        self.lifespan = {"Human": 70, "Dwarf": 120, "Elf": 1000, "Goblin": 25, "Orc": 50}[race]
        self.name = name
        self.title = title
        self.pos = pos
        self.origin = pos
        self.age = 0
        self.reputation = 0
        self.inventory = {}
        self.skills = {}
        self.alive = True
        self.birth = birth
        self.life = []
        self.pronoun = random.choice([["he", "him", "his", "his"], ["she", "her", "her", "hers"],
                                      ["they", "them", "their", "theirs"]])

    def tick(self):
        if not self.alive:
            return
        self.age += 1
        if (self.age - self.lifespan) / self.lifespan > random.random():
            self.life.append(DeathEvent(self, current_year, "old age"))
            self.alive = False
            if self.pos == self.origin:
                WorldMap[self.origin]["Structure"].resources = \
                    {{add_inv('self.inventory', 'WorldMap[self.origin]["Structure"].resources')}}
                self.inventory = {}
            return

        # Traveling
        traveler_options = [
            *(k for k in get_adj(self.pos, bounds=WorldMap.keys()) if
              (abs(k[0] - self.origin[0]) + abs(k[1] - self.origin[1])) < 10)]
        if self.origin in traveler_options and random.random() < (50 - self.age) / self.age:
            traveler_options.remove(self.origin)

        if self.pos != self.origin and traveler_options != []:
            self.pos = random.choice(traveler_options)  # Travel to a random location within the region
            if self.pos == self.origin:
                self.life.append(LifeEvent(self, current_year, " stopped traveling"))
                return
            for monster in (_.monster for _ in RegionList):
                if self.pos == monster.location and monster.alive:  # Combat
                    luck = random.random()
                    result = luck / (1 - luck) + {{getinv('self.skills', '"Adventuring"')}}
                    if {{getinv('self.skills', 'Magic["Name"]')}} > 0:
                        for k in Magic["Abilities"]:
                            if k["Type"] != "Combat":
                                continue
                            if k['Component'] == "None":
                                result += k["Strength"]
                                continue
                            res_types = [k["Component"]]
                            if k["Component"] == "Gem":
                                res_types = [*(f"Cut {k}" for k in Gems.keys())]
                            if k["Component"] == "Metal":
                                res_types = [*(f"{k} Goods" for k in Metals.keys())]
                            ability_res = [*shuffle_dict({k: v for k, v in self.inventory.items()
                                                          if k in res_types and v > 0}).items()]
                            if len(ability_res) > 0:
                                if ability_res[0][1] <= 0:
                                    continue
                                amount = min(k["Strength"], ability_res[0][1])
                                {{inv_opeq('self.inventory', 'ability_res[0][0]', '-', 'amount')}}
                                # print(ability_res[0][0], ability_res[0][1])
                                result += amount
                    self.reputation += 2
                    if result < 8:  # Monster kills NPC
                        self.alive = False
                        monster.inventory = {{add_inv('self.inventory', 'monster.inventory')}}
                        self.inventory = {}
                        self.life.append(DeathEvent(self, current_year, "a battle with " + monster.name))
                        break
                    elif result < 25:  # NPC escapes Monster
                        self.life.append(LifeEvent(self, current_year, " fought " + monster.name + " and escaped"))
                        self.skills['Adventuring'] += 1
                        if self.skills['Adventuring'] == 10:
                            self.life.append(LifeEvent(self, current_year, f"became a master in adventuring"))
                        elif self.skills['Adventuring'] == 20:
                            self.life.append(LifeEvent(self, current_year, f"became an expert in adventuring"))
                    else:  # NPC defeats Monster
                        self.life.append(LifeEvent(self, current_year, " defeated " + monster.name))
                        for city in [*(WorldMap[c]["Structure"] for c in CityList if
                                       c in RegionList[RegionMap[self.pos]].tiles)]:
                            city.history.append(
                                CityEvent(city, current_year, f" celebrated the defeat of {monster.name} by {self}"))
                        self.inventory = {{add_inv('self.inventory', 'monster.inventory')}}
                        monster.inventory = {}
                        monster.alive = False
                        self.reputation += 2
            else:
                if self.pos in CityList:  # Enter another city
                    my_city = WorldMap[self.origin]["Structure"]
                    other_city = WorldMap[self.pos]["Structure"]
                    culture_distance = {{cultural_distance("my_city", "other_city")}}
                    cultural_differences = {k: abs(v - other_city.cultural_values[k]) for k, v in
                                            my_city.cultural_values.items()}
                    biggest_difference = max(cultural_differences.values())
                    if biggest_difference < 1:  # If the cultures are too similar, don't change them at all
                        biggest_difference = -1
                    for k, v in my_city.cultural_values.items():
                        if cultural_differences[k] != biggest_difference:
                            continue
                        difficulty = culture_distance * cultural_differences[k] - \
                                     {{getinv('self.skills', '"Philosophy"')}}
                        luck = random.random()
                        if luck / (1 - luck) > difficulty:  # This can be anywhere from 0 to infinity
                            other_city.cultural_values[k] = my_city.cultural_values[k]
                            self.life.append(LifeEvent(self, current_year,
                                                       f"convinced {other_city.name} to {'reject' if my_city.cultural_values[k] < 3 else 'accept' if my_city.cultural_values[k] > 3 else 'become neutral to'} {k}"))
                            other_city.history.append(CityEvent(other_city, current_year,
                                                                f"was convinced to {'reject' if my_city.cultural_values[k] < 3 else 'accept' if my_city.cultural_values[k] > 3 else 'become neutral to'} {k}"))
                        break
            return

        if self.age > 15 and random.random() * 10 < (
                {{getinv('self.skills', '"Adventuring"')}} / self.age) and traveler_options != []:
            self.pos = random.choice(traveler_options)  # Travel to a random location within the region
            self.life.append(LifeEvent(self, current_year, " began traveling"))
            self.reputation += 1
            # Bring Magic Items With
            requested_magic = {}
            for k in Magic["Abilities"]:
                if k["Component"] == "None":
                    continue
                if k["Type"] == "Combat":
                    res_types = [k["Component"]]
                    if k["Component"] == "Gem":
                        res_types = [*(f"Cut {k}" for k in Gems.keys())]
                    if k["Component"] == "Metal":
                        res_types = [*(f"{k} Goods" for k in Metals.keys())]
                    ability_res = [*shuffle_dict({k: v for k, v in self.inventory.items()
                                                  if k in res_types}).items()]
                    if len(ability_res) > 0:
                        {{inv_opeq('requested_magic', 'ability_res[0][1]', '+', 'k["Strength"]')}}
                        print(ability_res[0][0], ability_res[0][1])
            for k in requested_magic.keys():
                {{inv_opeq('requested_magic', 'k', '-', getinv('self.inventory', 'k'))}}
                amount = min(requested_magic[k], {{getinv('WorldMap[self.origin]["Structure"].resources', 'k')}})
                if amount > 0:
                    {{inv_opeq('self.inventory', 'k', '+', 'amount')}}
                    {{inv_opeq('WorldMap[self.origin]["Structure"].resources', 'k', '-', 'amount')}}
            return

        # Learning / Studying
        if self.age > 15:
            study_choices = {k: 0.1 + {{getinv('self.skills', 'k')}} % 12
                             for k in ["Leadership", "Metalworking", "Teaching", "Gemcutting", Magic["Name"],
                                       "Animal Training", "Adventuring", "Philosophy"]}
            knowledge_weight = WorldMap[self.origin]["Structure"].cultural_values["Knowledge"]
            for k in ["Leadership", "Teaching", Magic["Name"], "Philosophy"]:
                study_choices[k] += knowledge_weight
            art_weight = WorldMap[self.origin]["Structure"].cultural_values["Art"]
            for k in ["Metalworking", "Gemcutting", "Animal Training"]:
                study_choices[k] += art_weight
            study_choices["Adventuring"] += WorldMap[self.origin]["Structure"].cultural_values["Might"]
            total_studies = sum(study_choices.values())
            study_choices = {k: v / total_studies for k, v in study_choices.items()}
            study = random.choices([*study_choices.keys()], [*study_choices.values()])[0]
            idol = 0
            library = WorldMap[self.pos]['Structure'].library
            write = False
            if study in library.keys():
                for k, v in library[study].items():
                    if int(k) <= {{getinv('self.skills', 'study')}}:
                        continue
                    idol = v / (int(k) - {{getinv('self.skills', 'study')}})
                    break
                else:
                    write = True
            else:
                write = True
            difficulty = (self.age ** 2) * {{getinv('self.skills', 'study')}} / 100 / \
                         max(idol - {{getinv('self.skills', 'study')}}, 10)
            luck = random.random()
            if luck / (1 - luck) > difficulty:  # This can be anywhere from 0 to infinity
                {{inv_opeq('self.skills', 'study', '+', '1')}}
                if self.skills[study] == 2:
                    self.reputation += 1
                    self.life.append(LifeEvent(self, current_year, f"began studying {study.lower()}"))
                elif self.skills[study] == 10:
                    self.reputation += 1
                    self.life.append(LifeEvent(self, current_year, f"became a master in {study.lower()}"))
                elif self.skills[study] == 20:
                    self.reputation += 1
                    self.life.append(LifeEvent(self, current_year, f"became an expert in {study.lower()}"))
                if write:
                    if study in library.keys():
                        if {{getinv('library[study]', 'str(self.skills[study])')}} < \
                                {{getinv('self.skills', '"Teaching"')}}:
                            library[study][str(self.skills[study])] = self.skills["Teaching"]
                    else:
                        library[study] = {self.skills[study]: {{getinv('self.skills', '"Teaching"')}}}

            # Using productive skills
            metalwork_prod = {{getinv('self.skills', '"Metalworking"')}}
            # Based on population stuff, a single person shouldn't
            # be able to make this much, but I'm saying this number includes their students and coworkers
            if metalwork_prod > 0:
                metalwork_res = shuffle_dict({k: v for k, v in WorldMap[self.pos]['Structure'].resources.items()
                                              if k in Metals.keys()})
                for k, v in metalwork_res.items():  # Metalworking
                    current_goods = min(metalwork_prod, math.floor(v))
                    assert current_goods >= 0, f"{self.name} was about to make {current_goods} {k} Goods. Somethin' ain't right. Prod={metalwork_prod}  res={math.floor(v)}"
                    metalwork_prod -= current_goods
                    {{inv_opeq('WorldMap[self.origin]["Structure"].resources', 'k', '-', 'current_goods')}}
                    {{inv_opeq('WorldMap[self.origin]["Structure"].resources', 'k + " Goods"', '+', 'current_goods')}}
                    {{inv_opeq('WorldMap[self.origin]["Structure"].production', 'k + " Goods"', '+', 'current_goods')}}
                    if metalwork_prod <= 0:
                        break

            gemwork_prod = {{getinv('self.skills', '"Gemcutting"')}}
            if gemwork_prod > 0:
                gemwork_res = shuffle_dict({k: v for k, v in WorldMap[self.pos]['Structure'].resources.items()
                                            if k in Gems.keys()})
                for k, v in gemwork_res.items():  # Gemcutting
                    current_goods = min(gemwork_prod, math.floor(v))
                    assert current_goods >= 0, f"{self.name} was about to make {current_goods} Cut {k}. Somethin' ain't right. Prod={gemwork_prod}  res={math.floor(v)}"
                    gemwork_prod -= current_goods
                    {{inv_opeq('WorldMap[self.origin]["Structure"].resources', 'k', '-', 'current_goods')}}
                    {{inv_opeq('WorldMap[self.origin]["Structure"].resources', '"Cut " + k', '+', 'current_goods')}}
                    {{inv_opeq('WorldMap[self.origin]["Structure"].production', '"Cut " + k', '+', 'current_goods')}}
                    if gemwork_prod <= 0:
                        break

            animal_prod = {{getinv('self.skills', '"Animal Training"')}}
            if animal_prod > 0:
                animal_res = shuffle_dict({k: v for k, v in WorldMap[self.pos]['Structure'].resources.items()
                                           if k in Animals.keys()})
                for k, v in animal_res.items():  # Animal Training
                    current_goods = min(animal_prod, math.floor(v) / Animals[k][3])
                    assert current_goods >= 0, f"{self.name} was about to make {current_goods} Tame {k}. Somethin' ain't right. Prod={animal_prod}  res={math.floor(v)}"
                    animal_prod -= current_goods
                    {{inv_opeq('WorldMap[self.origin]["Structure"].resources', 'k', '-', 'current_goods')}}
                    {{inv_opeq('WorldMap[self.origin]["Structure"].resources', '"Tame " + k', '+', 'current_goods')}}
                    {{inv_opeq('WorldMap[self.origin]["Structure"].production', '"Tame " + k', '+', 'current_goods')}}
                    if animal_prod <= 0:
                        break

            magic_prod = {{getinv('self.skills', 'Magic["Name"]')}}
            if magic_prod > 0:
                magic_types = [*(k for k in Magic["Abilities"] if k["Type"] in ["Homunculus", "Youth", "Portal"]
                                 and self.skills[Magic["Name"]] > {{getinv('k', '"Min Level"')}})]
                random.shuffle(magic_types)
                # print(magic_types)
                for k in magic_types:  # Magic
                    res_types = [k["Component"]]
                    if k["Component"] == "Gem":
                        res_types = [*(f"Cut {k}" for k in Gems.keys())]
                    if k["Component"] == "Metal":
                        res_types = [*(f"{k} Goods" for k in Metals.keys())]
                    ability_res = shuffle_dict({k: v for k, v in WorldMap[self.pos]['Structure'].resources.items()
                                                if k in res_types})
                    # print(ability_res)
                    for kk, vv in ability_res.items():
                        current_goods = min(magic_prod, math.floor(vv / k["Strength"]))
                        assert current_goods >= 0, f"{self.name} was about to perform {current_goods} {['Type']}. Somethin' ain't right. Prod={magic_prod}  res={math.floor(vv / k['Strength'])}"
                        # print(magic_prod, math.floor(vv / k["Strength"]))
                        if k["Type"] == "Portal":
                            if "Portal" in WorldMap[self.origin]["Structure"].artifacts:
                                continue
                            if current_goods < 1:
                                continue
                            current_goods = 1
                        magic_prod -= current_goods * k["Strength"]
                        {{inv_opeq('WorldMap[self.origin]["Structure"].resources', 'k["Component"]', '-',
                                   'current_goods * k["Strength"]')}}
                        if k["Type"] == "Homunculus":
                            WorldMap[self.origin]["Structure"].homunculi += current_goods * k["Strength"]
                            # print("made", current_goods * k["Strength"], "homunculi")
                        elif k["Type"] == "Youth":
                            self.age -= current_goods * k["Strength"] / 10
                        elif k["Type"] == "Portal":
                            cities = CityList.copy()
                            random.shuffle(cities)
                            for k in cities:
                                if "Portal" in WorldMap[k]["Structure"].artifacts and (
                                        self.origin, k) not in trade_connections.keys() and (
                                        k, self.origin) not in trade_connections.keys():
                                    trade_connections[(self.origin, k)] = 0
                                    WorldMap[k]["Structure"].artifacts.remove("Portal")
                            else:
                                WorldMap[self.origin]["Structure"].artifacts.append("Portal")
                            self.life.append(LifeEvent(self, current_year,
                                                       f"established a portal in {WorldMap[self.origin]['Structure'].name}"))
                        if magic_prod <= 0:
                            break
                    if magic_prod <= 0:
                        break

    def __str__(self):
        return f"{self.name}, {self.race} {self.title} of {WorldMap[self.origin]['Structure'].name}"

    def jsonize(self):
        return {"name": self.name, "race": self.race, "title": self.title, "pos": self.pos, "birth": self.birth,
                "alive": self.alive, "inventory": self.inventory, "skills": self.skills, "origin": self.origin,
                "age": self.age, "life": [*(e.jsonize() for e in self.life)], "reputation": self.reputation}


class LifeEvent:
    def __init__(self, npc, time, desc=""):
        self.time = time
        self.desc = desc
        self.npc = npc

    def __str__(self):
        return f"In Y{self.time}, {self.npc.pronoun[0]} {self.desc}."

    def longstr(self):
        return f"In Y{self.time}, {self.npc}, {self.desc}."

    def jsonize(self):
        return {"Type": "Event", "Time": self.time, "Desc": self.desc}


class CityEvent(LifeEvent):
    def __str__(self):
        return f"In Y{self.time}, {self.npc.name} {self.desc}."

    def longstr(self):
        return str(self)


class DeathEvent(LifeEvent):
    def __str__(self):
        return f"In Y{self.time}, {self.npc.name} died at age {self.time - self.npc.birth} because of {self.desc}."

    def longstr(self):
        return f"In Y{self.time}, {self.npc}, died at age {self.time - self.npc.birth} because of {self.desc}."

    def jsonize(self):
        return {"Type": "Death", "Time": self.time, "Desc": self.desc}


class Monster:
    def __init__(self, region):
        if region is None:
            return
        self.alive = True
        self.location = None
        self.inventory = {}
        self.species = random.choice(Biomes[region.terrain]["Monsters"])
        self.name = names.generate("Monster", max_length=15)
        mon_color = random.choice(['red', 'blue', 'green', 'black', 'white',
                                   *(k.lower() for k in region.resources.keys() if
                                     k in [*Gems.keys(), *Metals.keys()])])
        if mon_color in [*Gems.keys(), *Metals.keys()]:
            self.inventory[mon_color] = 20
        if self.species == "Dragon":
            self.desc = f"a great winged reptile with {random.choice(['engraved', 'long', 'sharpened', 'serrated'])} " \
                        f"horns and claws and {mon_color} scales. It " + \
                        random.choice(['is engraved with symbols', 'has a prominent scar', 'wears bone jewelry'])
        elif self.species == "Worm":
            self.desc = f"an enormous worm with {mon_color} plating " + \
                        random.choice(
                            ['engraved with symbols', 'covered in spikes', 'and a fleshy sail along its spine'])
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
        return {"name": self.name, "species": self.species, "desc": self.desc, "inventory": self.inventory,
                "alive": self.alive, "location": self.location}

    def tick(self, region):
        if not self.alive:
            return
        if self.location is None:
            self.location = random.choice(region.tiles)
        else:
            try:
                self.location = random.choice(
                    [*(k for k in get_adj(self.location, bounds=WorldMap.keys()) if k in region.tiles)])
            except IndexError:  # If the sequence is empty, don't bother moving or doing anything
                pass
        if self.location in CityList:  # Raid a city
            city = WorldMap[self.location]['Structure']
            CityRes = {k: v * (Goods[k][1] if k in Goods.keys() else (
                    4 * (Goods[{{extract_material('k')}}][1] if {{extract_material('k')}} in Goods.keys() else 1)))
                       for k, v in sorted(city.resources.items(), key=lambda item: item[1], reverse=True)}
            # print(city.resources, CityRes)
            # city.history.append(CityEvent(city, current_year, f"was raided by {self.name}"))
            for i in range(min(3, len(CityRes))):
                res = [*CityRes.keys()][i]
                amount = min(city.resources[res], 20)
                {{inv_opeq('city.resources', 'res', '-', 'amount')}}
                {{inv_opeq('self.inventory', 'res', '+', 'amount')}}
                # print(self.name, "stole", amount, res, "from", city.name)


def roundrand():
    return int(random.random() * 1000) / 1000


def shuffle_dict(_i):
    keys = [*(i for i in _i.keys())]
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
            pygame.quit()
            export_report()
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

    population_average = sum([1, *(WorldMap[k]["Structure"].population for k in CityList)]) / (len(CityList) + 1)

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

    for k in CityList:  # Draw circles for traveling NPCs
        c = WorldMap[k]['Structure']
        for npc in c.npcs:
            if not npc.alive:
                continue
            # print(npc.name, npc.pos, npc.origin, npc.skills)
            if npc.pos == npc.origin:
                continue
            x = 10 * npc.pos[0] + 25
            y = 10 * npc.pos[1] + 25
            pygame.draw.circle(Display, (128, 255, 128, 128), (x, y), 3)
        for a in c.armies:
            if a["Size"] <= 0:
                continue
            if a["Pos"] == a["Origin"]:
                continue
            x = 10 * a["Pos"][0] + 25
            y = 10 * a["Pos"][1] + 25
            pygame.draw.circle(Display, (128, 128, 255, 128), (x, y), 3)

    for r in RegionList:  # Draw circles for traveling monsters
        if not r.monster.alive:
            continue
        x = 10 * r.monster.location[0] + 25
        y = 10 * r.monster.location[1] + 25
        pygame.draw.circle(Display, (255, 128, 128, 128), (x, y), 3)

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


def get_adj(pos, bounds=None, r=1):
    adj = []
    if r == 0:
        for a in [(0, 1), (1, 0), (0, -1), (-1, 0)]:
            adj.append((a[0] + pos[0], a[1] + pos[1]))
        return adj
    for x in range(2 * r + 1):
        for y in range(2 * r + 1):
            if (x, y) == (r, r):
                continue
            if bounds is None or (pos[0] + x - r, pos[1] + y - r) in bounds:
                adj.append((pos[0] + x - r, pos[1] + y - r))
    return adj


def manage_trade(route):
    c1 = WorldMap[route[0]]["Structure"]
    c2 = WorldMap[route[1]]["Structure"]
    c1_demand = {}  # c1 trade => gift to c1
    c2_demand = {}  # c2 trade => gift to c2
    for k in c1.economy.keys():
        if c1.economy[k] <= 0:
            continue
        c1_demand[k] = config["TRADE_VOLUME"] / c1.economy[k]
    for k in c2.economy.keys():
        if c2.economy[k] <= 0:
            continue
        c2_demand[k] = config["TRADE_VOLUME"] / c2.economy[k]
    c1_keys = list(dict.fromkeys([*c1_demand.keys(), *c2.economy.keys()]))
    c2_keys = list(dict.fromkeys([*c2_demand.keys(), *c1.economy.keys()]))
    c1_supply = {k: {{getinv('c1_demand', 'k')}} * {{getinv('c2.economy', 'k')}} for k in c1_keys}
    c2_supply = {k: {{getinv('c2_demand', 'k')}} * {{getinv('c1.economy', 'k')}} for k in c2_keys}
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
    if {{cultural_distance('c1', 'c2')}} > 50:
        return
    # TODO: Something about sharing culture
    if len(c1_supply) == 0 or len(c2_supply) == 0:
        return
    try:
        c1_min = min(c1_supply.values())
        c1_trade = \
            [(key, value, math.floor(c1_demand[key])) for key, value in c1_supply.items() if value == c1_min][0]
        c2_min = min(c2_supply.values())
        c2_trade = \
            [(key, value, math.floor(c2_demand[key])) for key, value in c2_supply.items() if value == c2_min][0]
        if c1_trade[1] > (config["TRADE_VOLUME"] - config["TRADE_THRESHOLD"]) or c2_trade[1] > (
                config["TRADE_VOLUME"] - config["TRADE_THRESHOLD"]) or \
                {{getinv('c1.resources', 'c2_trade[0]')}} < c2_trade[2] or {{getinv('c2.resources', 'c1_trade[0]')}} < \
                c1_trade[2]:
            return
        {{inv_opeq('c1.resources', 'c1_trade[0]', "+", "c1_trade[2]")}}
        {{inv_opeq('c1.imports', 'c1_trade[0]', "+", "c1_trade[2]")}}
        {{inv_opeq('c2.resources', 'c1_trade[0]', "-", "c1_trade[2]")}}
        {{inv_opeq('c2.imports', 'c1_trade[0]', "-", "c1_trade[2]")}}

        {{inv_opeq('c2.resources', 'c2_trade[0]', "+", "c2_trade[2]")}}
        {{inv_opeq('c2.imports', 'c2_trade[0]', "+", "c2_trade[2]")}}
        {{inv_opeq('c1.resources', 'c2_trade[0]', "-", "c2_trade[2]")}}
        {{inv_opeq('c1.imports', 'c2_trade[0]', "-", "c2_trade[2]")}}

        c1.trade.append({"to": route[1], "give": f"{c2_trade[2]} {c2_trade[0]}", "get": f"{c1_trade[2]} {c1_trade[0]}",
                         "year": current_year})
        c2.trade.append({"to": route[0], "give": f"{c1_trade[2]} {c1_trade[0]}", "get": f"{c2_trade[2]} {c2_trade[0]}",
                         "year": current_year})

        trade_connections[route] += 1
        # print(f"{c1.name} and {c2.name} Trade {c1_trade[2]} {c1_trade[0]} for {c2_trade[2]} {c2_trade[0]}.")
    except IndexError:
        return


def build_region_map(config_=None):
    if RegionMap:
        return {k: None for k in RegionMap.keys()}, RegionMap, max(RegionMap.values()) + 1, RegionList
    import random
    Regions = 0
    WorldMap_ = {}
    RegionMap_ = {}
    for x in range(config_["WORLD_SIZE"][0]):  # Set up coords
        for y in range(config_["WORLD_SIZE"][1]):
            WorldMap_[(x, y)] = None
            if x in [0, config_["WORLD_SIZE"][0] - 1] or y in [0, config_["WORLD_SIZE"][1] - 1]:
                RegionMap_[(x, y)] = 0
            else:
                RegionMap_[(x, y)] = None

    keys = [*RegionMap_.keys()]

    while True:
        random.shuffle(keys)
        for k in keys:
            if RegionMap_[k] is not None:
                continue
            for n in range(config_["GEN_RADIUS"]):  # Look for adjacent tiles before starting a new Region
                adjacent_terrain = [*(RegionMap_[a] for a in get_adj(k, r=n, bounds=RegionMap_.keys())
                                      if RegionMap_[a] is not None)]
                if adjacent_terrain:
                    RegionMap_[k] = random.choice(adjacent_terrain)
                    break
            else:  # Start a new Region
                if random.random() < config_["SIZE_PARAMETER"]:
                    continue
                Regions += 1
                RegionMap_[k] = Regions
        for k in keys:
            if RegionMap_[k] is None:
                break
        else:
            break

    RegionList_ = [Region(terrain="Ocean")]  # Generate the Regions
    for r in range(Regions):
        RegionList_.append(Region())

    return WorldMap_, RegionMap_, Regions, RegionList_


def generate_map():
    global WorldMap, RegionList, RegionMap

    WorldMap, RegionMap, Regions, RegionList = build_region_map(config)

    for k in RegionMap.keys():  # Apply the Regions' terrains
        WorldMap[k] = {"Terrain": RegionList[RegionMap[k]].terrain}
        RegionList[RegionMap[k]].tiles.append(k)

    for k in RegionMap.keys():  # Find adjacent Regions
        adj = get_adj(k, bounds=RegionMap.keys())
        for a in adj:
            if RegionMap[a] not in RegionList[RegionMap[k]].adjacent_regions and RegionMap[a] != RegionMap[k]:
                RegionList[RegionMap[k]].adjacent_regions.append(RegionMap[a])


def generate_cities():
    keys = WorldMap.keys()
    possible_cities = []
    for k in keys:
        if WorldMap[k]["Terrain"] in ["Ocean", "Sea"]:
            continue
        adj = get_adj(k, bounds=WorldMap.keys())
        adj_terrain = [*(WorldMap[a]["Terrain"] for a in adj)]
        if "Ocean" in adj_terrain or "Sea" in adj_terrain:
            if random.random() < config["COASTAL_CITY_DENSITY"] * (
                    config["MOUNTAIN_CITY_DENSITY"] if WorldMap[k]["Terrain"] in ["Mountain", "Desert"] else 1):
                possible_cities.append(k)
        else:
            if random.random() < config["INLAND_CITY_DENSITY"] * (
                    config["MOUNTAIN_CITY_DENSITY"] if WorldMap[k]["Terrain"] in ["Mountain", "Desert"] else 1):
                possible_cities.append(k)
    random.shuffle(possible_cities)
    for p in possible_cities:
        adj = get_adj(p, bounds=WorldMap.keys())
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
    # Find trade connections
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
    if Magic["Material"][0] != "":
        yield
        yield
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
    Magic["Name"] = names.generate("Magic")
    yield  # Run the second call after regions are generated
    if Magic['Localization'] == "Localized":  # Get rid of all of the resource other than one random location
        select_regions = [r for r in RegionList if
                          hasattr(r, "resources") and {{getinv('r.resources', 'Magic["Material"][0]')}} > 0]
        cl = CityList.copy()
        random.shuffle(cl)
        for c in CityList:
            city = WorldMap[c]["Structure"]
            if city.region in select_regions:
                select_regions.remove(city.region)
        for r in select_regions:
            r.resources[Magic["Material"][0]] = 0
    # NOTE: Make sure to add these in the NPC magic-using section
    Magic["Abilities"] = []
    # Increase life span
    if random.random() < 0.6:
        Magic["Abilities"].append({"Type": "Youth", "Component": random.choice(["Gem", "Metal", Magic["Material"][0]]),
                                   "Strength": random.randint(2, 6), "Min Level": 2})
    # Homunculi - increase population for purposes of resource production
    if random.random() < 0.4:
        Magic["Abilities"].append(
            {"Type": "Homunculus", "Component": random.choice(["Gem", "Metal", Magic["Material"][0]]),
             "Strength": random.randint(2, 5) * 10, "Min Level": 2})
    # Combat score
    Magic["Abilities"].append({"Type": "Combat", "Component": random.choice(["Gem", "Metal", Magic["Material"][0]]),
                               "Strength": random.randint(2, 6), "Min Level": 2})
    # Lichdom - teleport home on death
    # Teleportation - long-range trade routes
    if random.random() < 0.4:
        Magic["Abilities"].append({"Type": "Portal", "Component": random.choice(["Gem", "Metal", Magic["Material"][0]]),
                                   "Strength": random.randint(2, 5) * 100, "Min Level": 10})
    print(Magic)
    yield


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
        "Range": ((median([0, start[0], config["WORLD_SIZE"][0]]), median([0, start[1], config["WORLD_SIZE"][1]])),
                  (median([0, end[0], config["WORLD_SIZE"][0]]), median([0, end[1], config["WORLD_SIZE"][1]]))),
        "Selection": selection,
        "Scale": scale,
        "Colors": {k: "#" + hex(sum(i * (256 ** (2 - ii)) for ii, i in enumerate(v["Color"])))[2:] for k, v in
                   Biomes.items()},
        "TradeRoutes": [*(k for k in trade_connections.keys() if trade_connections[k] > 1)] if trade else [],
        "min": min,
        "abs": abs
    }
    return results_template.render(context)


def export_report():
    # print(json.dumps(jsonize()))
    environment = Environment(loader=FileSystemLoader("templates/"))
    results_filename = f"reports/{Magic['Material'][0]}_{Magic['Name']}_{current_year}.html"

    with open(f"json_data/{Magic['Material'][0]}_{Magic['Name']}_{current_year}.json", "w") as json_dump:
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
        "WORLD_SIZE": config["WORLD_SIZE"],
        "MagicStr": {Magic["Material"][0]: f"{Magic['Material'][0]} is", "Gem": "Cut gems are",
                     "Metal": "Metal goods are", "Youth": "restoring youth", "Homunculus": "creating homunculi",
                     "Combat": "aiding in combat", "Portal": "creating portals"}
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
            "Config": config}


pygame.font.init()
font_size = 20
font = pygame.font.Font('PTM55FT.ttf', font_size)
Display = None


def sim_step():
    global current_year

    # with Pool() as pool:
    #     pool.imap(lambda key: WorldMap[key]["Structure"].tick, CityList, chunksize=10)
    for key in CityList:
        WorldMap[key]["Structure"].tick()
    for _ in range(config["TRADE_QUANTITY"]):
        manage_trade(random.choice([*trade_connections.keys()]))
    for region in RegionList:
        region.tick()
    current_year += 1


def simulate():
    while True:
        sim_step()
        update_screen()


def init_magic():
    AllItems.append(Magic["Material"][0])
    if Magic["Material"][2] == "Gemstone":
        Gems[Magic["Material"][0]] = Magic["Material"][1]
        AllItems.append(f"Cut {Magic['Material'][0]}")
    elif Magic["Material"][2] == "Metal":
        Metals[Magic["Material"][0]] = Magic["Material"][1]
        AllItems.append(f"{Magic['Material'][0]} Goods")
    elif Magic["Material"][2] == "Plant":
        Plants[Magic["Material"][0]] = Magic["Material"][1]


def pregen(pregen_length):
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
            if txt == "HISTORY_EVENT":
                for _ in range(10):
                    city = WorldMap[random.choice(CityList)]["Structure"]
                    if len(city.history) == 0:
                        npc = random.choice(city.npcs)
                        if len(npc.life) != 0:
                            txt = random.choice(npc.life).longstr()
                    else:
                        txt = random.choice(city.history).longstr()
            if txt == "HISTORY_EVENT":
                txt = "The world is new"
            pygame.draw.rect(Display, (96, 96, 96), (250, 400, 500, 500))
            blit_txt(txt, 250, 400, Display, maxlength=42)
            pygame.display.update((250, 400, 500, 500))
        for event in pygame.event.get():
            if event.type == QUIT:
                pygame.quit()
                sys.exit()
        for _ in range(pregen_length):
            sim_step()


def init():
    global Display

    pygame.init()
    Display = pygame.display.set_mode((900, 700))
    pygame.display.set_caption('Continent')


def init_world():
    magic_generator = generate_magic()
    next(magic_generator)
    init_magic()
    generate_map()
    next(magic_generator)
    set_races()
    generate_cities()
    pregen(config["PREGEN_LENGTH"])
