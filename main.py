# Load data from a .json file, and run sim.py

import sys
import json
import ast

import sim


def dejsonize_npc(npc):
    npc_return = sim.NPC(npc["race"], npc["name"], tuple(npc["origin"]), npc["birth"], npc["title"])
    npc_return.pos = tuple(npc["pos"])
    for attr_key in ["inventory", "skills", "alive", "reputation", "age"]:
        setattr(npc_return, attr_key, npc[attr_key])
    npc_return.life = [*((sim.LifeEvent(npc_return, e["Time"], e["Desc"]) if e["Type"] == "Event" else
                          sim.DeathEvent(npc_return, e["Time"], e["Desc"])) for e in npc["life"])]
    npc_return.age = sim.current_year - npc["birth"]
    return npc_return


def dejsonize_city(c):
    city_return = sim.City(tuple(c["pos"]))
    for attr_key in ["name", "population", "data", "trade", "resources", "resource_gathering", "economy", "imports",
                     "production", "library", "homunculi", "artifacts", "cultural_values"]:
        setattr(city_return, attr_key, c[attr_key])
    keylist = [*(int(k) for k in c["data"].keys())]
    keylist.sort()
    city_return.data = {str(k): c["data"][str(k)] for k in keylist}
    city_return.npcs = [dejsonize_npc(npc) for npc in c["NPCs"]]
    city_return.history = [*(sim.CityEvent(city_return, e["Time"], e["Desc"]) for e in c["history"])]
    return city_return


def dejsonize_monster(m):
    monster_return = sim.Monster(None)
    for attr_key in ["species", "name", "desc", "inventory", "alive"]:
        setattr(monster_return, attr_key, m[attr_key])
    monster_return.location = tuple(m["location"])
    return monster_return


def dejsonize_region(r):
    region_return = sim.Region(terrain=r["terrain"])
    for attr_key in ["ancestor_race", "adjacent_regions", "demographics", "resources"]:
        setattr(region_return, attr_key, r[attr_key])
    region_return.tiles = [tuple(c) for c in r["tiles"]]
    region_return.monster = dejsonize_monster(r["monster"])
    return region_return


def extract_json(filename):
    with open(filename) as file:
        return json.loads(file.read())


def main(argv):
    patharg = "default"
    if len(argv) >= 2:
        patharg = argv[1]
        if argv[1] == "Help":
            print('>python main.py {filename} {flags}\n'
                  'P{#} - Run the simulation in speed mode for # * 500 years before displaying\n'
                  'R    - Don\'t run the simulation; instead, just generate a report')
            sys.exit()
    jsondict = patharg
    if isinstance(patharg, str):
        jsondict = extract_json(f"{patharg}.json")
    sim.Biomes = jsondict["Biomes"]
    print({k: "#" + hex(sum(i * (256 ** (2 - ii)) for ii, i in enumerate(v["Color"])))[2:] for k, v in
           sim.Biomes.items()})
    sim.config = jsondict["Config"]
    sim.init()
    if isinstance(jsondict["Items"], list):
        for item_file in jsondict["Items"]:
            with open(f"objects/{item_file}.txt") as file:
                line_iter = (i for i in file.readlines())
                try:
                    while line_iter:
                        line = next(line_iter)
                        split = line.split(":")
                        if len(split) == 2:
                            item_type = split[0]
                            name = split[1].strip()
                            data = tuple(float(i) for i in next(line_iter).split(","))
                            sim.Goods[name] = data
                            sim.AllItems.append(name)
                            if item_type == "animal":
                                sim.Animals[name] = data
                                sim.AllItems.append(f"Tame {name}")
                            elif item_type == "gem":
                                sim.Gems[name] = data
                                sim.AllItems.append(f"Cut {name}")
                            elif item_type == "metal":
                                sim.Metals[name] = data
                                sim.AllItems.append(f"{name} Goods")
                            elif item_type == "plant":
                                sim.Plants[name] = data
                except StopIteration:
                    pass  # If the file is over, don't care
    else:
        sim.Animals = jsondict["Items"]["Animals"]
        sim.Gems = jsondict["Items"]["Gems"]
        sim.Metals = jsondict["Items"]["Metals"]
        sim.Plants = jsondict["Items"]["Plants"]
        sim.Goods = {**sim.Metals, **sim.Gems, **sim.Plants, **sim.Animals}
        sim.AllItems = [*sim.Metals.keys(), *(f"{i} Goods" for i in sim.Metals.keys()), *sim.Gems.keys(),
                        *(f"Cut {i}" for i in sim.Gems.keys()),
                        *sim.Plants.keys(), *sim.Animals.keys(), *(f"Tame {i}" for i in sim.Animals.keys()), "Fish"]
    if jsondict["file_type"] == "save":
        sim.current_year = jsondict["current_year"]
        sim.Magic = jsondict["Magic"]
        sim.RegionList = [sim.Region(terrain="Ocean")]
        for i, r_ in enumerate(jsondict["RegionList"]):
            region = dejsonize_region(r_)
            sim.RegionList.append(region)
            for k in region.tiles:
                sim.WorldMap[k] = {"Terrain": region.terrain}
                sim.RegionMap[k] = i + 1
        for x in range(sim.config["WORLD_SIZE"][0]):
            for y in range(sim.config["WORLD_SIZE"][1]):
                if (x, y) in sim.WorldMap.keys():
                    continue
                sim.WorldMap[(x, y)] = {"Terrain": "Ocean"}
                sim.RegionMap[(x, y)] = 0
                sim.RegionList[0].tiles.append((x, y))
        for c_ in jsondict["CityList"]:
            city = dejsonize_city(c_)
            k = city.pos
            sim.WorldMap[k]['Structure'] = city
            sim.CityList.append(k)
        for k, v in jsondict["trade_connections"].items():
            k_l = ast.literal_eval(k)
            k_a = ((k_l[0], k_l[1]), (k_l[2], k_l[3]))
            sim.trade_connections[k_a] = v
        sim.init_magic()
    elif jsondict["file_type"] == "gen":
        sim.AllItems = ["Fish"]
        if "Magic" in jsondict.keys():
            sim.Magic = jsondict["Magic"]
        if "RegionList" in jsondict.keys():
            sim.RegionList = [sim.Region(terrain=r["terrain"]) for r in jsondict["RegionList"]]
            sim.RegionMap = jsondict["RegionMap"]
            for i, r in enumerate(jsondict["RegionList"]):
                sim.RegionList[i].tiles = r["tiles"]
            print("\n".join([*(str(k) for k in sim.RegionList)]))
        sim.init_world()
    if len(argv) >= 3:
        for arg in argv[2:]:
            if arg[0] == "P":
                sim.init()
                sim.pregen(int(arg[1:]))
    if "R" in argv:
        sim.export_report()
    else:
        sim.simulate()


if __name__ == "__main__":
    main(sys.argv)
