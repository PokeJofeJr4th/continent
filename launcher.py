# presents file choices and allows custom generation

import itertools
import math
import os
import json
import sys
import pygame
import compile
import copy
from pygame.locals import *

pygame.init()
Display = pygame.display.set_mode((900, 700))
pygame.display.set_caption('Continent Launcher')
pygame.font.init()
font_size = 20
font = pygame.font.Font('PTM55FT.ttf', font_size)

jsondata = []
options = []
filenames = []
argv = []
cursor = 0
report = False
pregen = 0


def blit_txt(txt, x, y, display, text_color=(255, 255, 255), maxlength=0):
    if "\n" in txt:
        text = txt.split("\n")
        y_ = 0
        for n in range(len(text)):
            y_ += blit_txt(text[n], x, y + y_ * 20, display,
                           text_color=text_color, maxlength=maxlength)
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


def custom_biomes(parameters, stack):
    global cursor
    change_direction = 0
    toggle = False
    for event in pygame.event.get():
        if event.type == pygame.QUIT:
            pygame.quit()
            sys.exit()
        elif event.type == pygame.KEYDOWN:
            if event.key == pygame.K_ESCAPE:
                cursor = 0
                return False
            elif event.key == pygame.K_RETURN:
                cursor = 0
                while True:
                    success = stack[0](parameters, stack[1:])
                    if not success:
                        break
            elif event.key == pygame.K_UP:
                cursor -= 1
                if cursor % 12 == 11:
                    cursor += 12
            elif event.key == pygame.K_DOWN:
                cursor += 1
                if cursor % 12 == 0:
                    cursor -= 12
            elif event.key == pygame.K_LEFT:
                cursor -= 12
                if math.floor(cursor / 12) < 0:
                    cursor = 12 * len(parameters["Biomes"]) - 12
            elif event.key == pygame.K_RIGHT:
                cursor += 12
                if math.floor(cursor / 12) >= len(parameters["Biomes"]):
                    cursor = 0
            elif event.key == pygame.K_KP_PLUS:
                change_direction = 1
            elif event.key == pygame.K_KP_MINUS:
                change_direction = -1
            elif event.key == pygame.K_t:
                toggle = True
    Display.fill((0, 0, 0))
    blit_txt("Custom Generation", 50, 10, Display)
    blit_txt("[Esc] Back", 400, 20, Display)
    blit_txt("[Enter] Continue", 400, 45, Display)
    i = 0
    i += blit_txt([*parameters["Biomes"].keys()][math.floor(cursor / 12)],
                  50, i * 22 + 80, Display, maxlength=60)
    r = [*parameters["Biomes"].values()][math.floor(cursor / 12)]
    if r["Resources"] == {}:
        i += blit_txt("No Resources", 50, i * 22 + 80, Display, maxlength=60)
    else:
        i += blit_txt("Resources:", 50, i * 22 + 80, Display, maxlength=60)
        for ii in range(5):
            if cursor % 12 == ii:
                blit_txt("[+/-] 0.1", 500, i * 22 + 80, Display)
                k = [*r["Resources"].keys()][ii]
                r["Resources"][k] = round(
                    r["Resources"][k] + change_direction * 0.1, 2)
                if r["Resources"][k] < 0:
                    r["Resources"][k] = 0
                if r["Resources"][k] > 1:
                    r["Resources"][k] = 1
            i += blit_txt(("  >" if cursor % 12 == ii else "   ") + [*r["Resources"].keys()][ii] + ": " + str(
                [*r["Resources"].values()][ii]), 50, i * 22 + 80,
                Display, maxlength=60)
    for ii, m in enumerate(["Beast", "Dragon", "Leviathan", "Worm"]):
        if cursor % 12 == ii + 5:
            blit_txt("[t] Toggle", 500, i * 22 + 80, Display)
            if toggle:
                if m in r["Monsters"]:
                    r["Monsters"].remove(m)
                else:
                    r["Monsters"].append(m)
        i += blit_txt((">" if cursor % 12 == ii + 5 else " ") + m, 50, i * 22 + 80,
                      Display, maxlength=60, text_color=((128, 255, 128) if m in r["Monsters"] else (255, 128, 128)))
    i += blit_txt("Color:", 50, i * 22 + 80, Display, maxlength=60)
    i += blit_txt("██████████", 50, i * 22 + 80, Display,
                  maxlength=60, text_color=r["Color"])
    for ii in range(3):
        if cursor % 12 == ii + 9:
            blit_txt("[+/-] 5", 500, i * 22 + 80, Display)
            r["Color"][ii] = r["Color"][ii] + change_direction * 5
            if r["Color"][ii] < 0:
                r["Color"][ii] = 0
            if r["Color"][ii] > 255:
                r["Color"][ii] = 255
        i += blit_txt(("  >" if cursor % 12 == ii + 9 else "   ") + ["R", "G", "B"][ii] + ": " + str(
            r["Color"][ii]), 50, i * 22 + 80, Display, maxlength=60)
    blit_txt(" ".join((f">{k}<" if iii == math.floor(cursor / 12) else f" {k} ")
                      for iii, k in enumerate([*parameters["Biomes"].keys()])), 50, 500, Display, maxlength=60)
    pygame.display.update()
    return True


def custom_items(parameters, stack):
    global cursor
    object_files = []
    toggle = False
    for _, _, files_ in os.walk("objects"):
        object_files = [*(file_[:-4] for file_ in files_)]
    for event in pygame.event.get():
        if event.type == pygame.QUIT:
            pygame.quit()
            sys.exit()
        elif event.type == pygame.KEYDOWN:
            if event.key == pygame.K_ESCAPE:
                cursor = 0
                return False
            elif event.key == pygame.K_RETURN:
                cursor = 0
                while True:
                    success = stack[0](parameters, stack[1:])
                    if not success:
                        break
            elif event.key == pygame.K_UP:
                cursor -= 1
                if cursor < 0:
                    cursor += len(object_files)
            elif event.key == pygame.K_DOWN:
                cursor += 1
                if cursor >= len(object_files):
                    cursor -= len(object_files)
            elif event.key == pygame.K_t:
                toggle = True
    Display.fill((0, 0, 0))
    blit_txt("Custom Generation", 50, 10, Display)
    blit_txt("[Esc] Back", 400, 20, Display)
    blit_txt("[Enter] Continue", 400, 45, Display)
    i = 0
    for ii, m in enumerate(object_files):
        if cursor == ii:
            blit_txt("[t] Toggle", 500, i * 22 + 80, Display)
            if toggle:
                if m in parameters["Items"]:
                    parameters["Items"].remove(m)
                else:
                    parameters["Items"].append(m)
        i += blit_txt((">" if cursor == ii else " ") + m, 50, i * 22 + 80,
                      Display, maxlength=60,
                      text_color=((128, 255, 128) if m in parameters["Items"] else (255, 128, 128)))
    pygame.display.update()
    return True


def custom_generation_parameters(parameters, stack):
    global cursor
    change_direction = 0
    for event in pygame.event.get():
        if event.type == pygame.QUIT:
            pygame.quit()
            sys.exit()
        elif event.type == pygame.KEYDOWN:
            if event.key == pygame.K_ESCAPE:
                cursor = 0
                return False
            elif event.key == pygame.K_UP:
                cursor -= 1
                if cursor == -1:
                    cursor = len(parameters["Config"].keys())
            elif event.key == pygame.K_DOWN:
                cursor += 1
                if cursor == len(parameters["Config"].keys()) + 1:
                    cursor = 0
            elif event.key == pygame.K_KP_PLUS:
                change_direction = 1
            elif event.key == pygame.K_KP_MINUS:
                change_direction = -1
            elif event.key == pygame.K_RETURN:
                cursor = 0
                while True:
                    success = stack[0](parameters, stack[1:])
                    if not success:
                        break
    Display.fill((0, 0, 0))
    blit_txt("Custom Generation", 50, 10, Display)
    blit_txt("[Esc] Back", 400, 20, Display)
    blit_txt("[Enter] Continue", 400, 45, Display)
    i = 0
    ii = -1
    for o in [*(parameters["Config"].keys())]:
        if o == "WORLD_SIZE":
            i += blit_txt(" WORLD_SIZE:", 30, i * 22 +
                          60, Display, maxlength=60)
            ii += 1
            if cursor == ii:
                blit_txt("[+/-] 10", 500, i * 22 + 60, Display)
                parameters["Config"]["WORLD_SIZE"][0] += change_direction * 10
            i += blit_txt(("  >" if cursor == ii else "   ") + str(parameters["Config"]["WORLD_SIZE"][0]), 30,
                          i * 22 + 60, Display, maxlength=60)
            ii += 1
            if cursor == ii:
                blit_txt("[+/-] 10", 500, i * 22 + 60, Display)
                parameters["Config"]["WORLD_SIZE"][1] += change_direction * 10
            i += blit_txt(("  >" if cursor == ii else "   ") + str(parameters["Config"]["WORLD_SIZE"][1]), 30,
                          i * 22 + 60, Display, maxlength=60)
            continue
        ii += 1
        if cursor == ii:
            change_quantity = {"GEN_RADIUS": 1, "SIZE_PARAMETER": 0.1, "COASTAL_CITY_DENSITY": 0.01,
                               "INLAND_CITY_DENSITY": 0.01, "MOUNTAIN_CITY_DENSITY": 0.1,
                               "MAX_PRODUCTION_CONSTANT": 100, "POPULATION_GROWTH_CONSTANT": 0.01,
                               "TRADE_THRESHOLD": 1, "TRADE_VOLUME": 10, "TRADE_QUANTITY": 5, "PREGEN_LENGTH": 1,
                               "MAGIC_CONSUMPTION": 0.01, "NOTABLE_NPC_THRESHOLD": 1, "NPC_COUNT": 100, "ARMY_SIZE": 10, "ARMY_PARAMETER": 0.001}[o]
            blit_txt("[+/-] " + repr(change_quantity),
                     500, i * 22 + 60, Display)
            parameters["Config"][o] += change_direction * change_quantity
            parameters["Config"][o] = round(parameters["Config"][o], 4)
        i += blit_txt((">" if cursor == ii else " ") + o + ": " + repr(parameters["Config"][o]), 30, i * 22 + 60,
                      Display, maxlength=60)
    pygame.display.update()
    return True


def build_world(parameters, stack):
    global cursor
    mouse_pos = pygame.mouse.get_pos()
    current_region = 0
    m_x = math.floor((mouse_pos[0] - 20) / 10)
    m_y = math.floor((mouse_pos[1] - 20) / 10)

    if "RegionMap" in parameters.keys():
        if (m_x, m_y) in parameters["RegionMap"].keys():
            current_region = parameters["RegionMap"][(m_x, m_y)]
    else:
        from sim import build_region_map
        _, parameters["RegionMap"], Regions, _ = build_region_map(
            parameters["Config"])
        parameters["RegionList"] = [*(None for _ in range(Regions+1))]

    for event in pygame.event.get():
        if event.type == pygame.QUIT:
            pygame.quit()
            sys.exit()
        elif event.type == pygame.KEYDOWN:
            if event.key == pygame.K_ESCAPE:
                cursor = 0
                return False
            elif event.key == pygame.K_LEFT:
                cursor -= 1
                if cursor < 0:
                    cursor += 1
            elif event.key == pygame.K_RIGHT:
                cursor += 1
                if cursor >= len(parameters["Biomes"]):
                    cursor -= 1
            elif event.key == pygame.K_RETURN:
                cursor = 0
                while True:
                    success = stack[0](parameters, stack[1:])
                    if not success:
                        break
        elif event.type == pygame.MOUSEBUTTONDOWN:
            parameters["RegionList"][current_region] = {"terrain": [
                *parameters["Biomes"].keys()][cursor], "tiles": []}
            for k, r in parameters["RegionMap"].items():
                if r == current_region:
                    parameters["RegionList"][current_region]["tiles"].append(k)

    Display.fill((0, 0, 0))
    for k, r in parameters["RegionMap"].items():
        xx = k[0] * 10 + 20
        yy = k[1] * 10 + 20
        tile_color = (24, 24, 24)
        if parameters["RegionList"][r]:
            tile_color = parameters["Biomes"][parameters["RegionList"]
                                              [r]["terrain"]]["Color"]
        elif (k[0] + k[1]) % 2 == 1:
            tile_color = (96, 96, 96)
        if r == current_region:
            tile_color = tuple(k / 2 + 127 for k in tile_color)
        pygame.draw.rect(Display, tile_color, (xx, yy, 10, 10))
    blit_txt(" ".join((f">{k}<" if iii == cursor else f" {k} ")
                      for iii, k in enumerate([*parameters["Biomes"].keys()])), 50, 500, Display, maxlength=60)
    pygame.display.update()

    # TODO: Generate from incomplete map
    return True


def go_to_main(parameters, stack):
    pygame.display.quit()
    import main
    argv = ["launcher", parameters]
    if report:
        argv.append("R")
    if pregen > 0:
        argv.append(f"P{pregen}")
    print(argv)
    main.main(argv)
    sys.exit()


def update_screen():
    global cursor, report, pregen
    for event in pygame.event.get():
        if event.type == pygame.QUIT:
            pygame.quit()
            sys.exit()
        elif event.type == pygame.KEYDOWN:
            if event.key == pygame.K_UP:
                cursor -= 1
                if cursor == -1:
                    cursor = len(options) - 1
            elif event.key == pygame.K_DOWN:
                cursor += 1
                if cursor == len(options):
                    cursor = 0
            elif event.key == pygame.K_KP_MINUS:
                if pregen != 0:
                    pregen -= 1
            elif event.key == pygame.K_KP_PLUS:
                pregen += 1
            elif event.key == pygame.K_r:
                report = not report
            elif event.key == pygame.K_c:
                parameters = copy.deepcopy(jsondata[cursor])
                if jsondata[cursor]["file_type"] == "save":
                    for k in ["RegionList", "CityList", "trade_connections", "current_year", "Magic"]:
                        del parameters[k]
                    parameters["file_type"] = "gen"
                stack = [custom_generation_parameters, custom_biomes,
                         build_world, custom_items, go_to_main]
                if jsondata[cursor]["file_type"] == "save":
                    stack.remove(custom_items)
                cursor = 0
                while True:
                    success = stack[0](parameters, stack[1:])
                    if not success:
                        break
            elif event.key == pygame.K_RETURN:
                pygame.quit()
                import main
                argv = ["launcher", filenames[cursor][:-5]]
                if report:
                    argv.append("R")
                if pregen > 0:
                    argv.append(f"P{pregen}")
                print(argv)
                main.main(argv)
                sys.exit()
    Display.fill((0, 0, 0))
    i = 0
    if report:
        blit_txt("[r] Generate a report", 50, 20, Display)
    else:
        blit_txt("[r] Run the simulation", 50, 20, Display)
    if pregen == 0:
        blit_txt("[+/-] no pregeneration", 50, 45, Display)
    else:
        blit_txt(f"[+/-] pregenerate {pregen * 500} years", 50, 45, Display)
    blit_txt("[Enter] Run", 400, 20, Display)
    blit_txt("[e] Edit regions", 400, 20, Display)
    terrain_colors = {}
    if jsondata[cursor]["file_type"] == "gen":
        blit_txt("[c] Customize", 400, 45, Display)
        for x in range(jsondata[cursor]["Config"]["WORLD_SIZE"][0]):
            for y in range(jsondata[cursor]["Config"]["WORLD_SIZE"][1]):
                if (x + y) % 2 == 0:
                    terrain_colors[(x, y)] = (96, 96, 96)
                else:
                    terrain_colors[(x, y)] = (24, 24, 24)
    else:
        blit_txt("[c] Copy generation parameters", 400, 45, Display)
        for r in jsondata[cursor]["RegionList"]:
            print(r)
            print(r["terrain"])
            terColor = jsondata[cursor]["Biomes"][r["terrain"]]["Color"]
            for t in r["tiles"]:
                terrain_colors[tuple(t)] = terColor
    oceanColor = jsondata[cursor]["Biomes"]["Ocean"]["Color"]
    for x in range(jsondata[cursor]["Config"]["WORLD_SIZE"][0]):
        xx = 50 + x * 3
        for y in range(jsondata[cursor]["Config"]["WORLD_SIZE"][1]):
            yy = 80 + y * 3
            if (x, y) in terrain_colors.keys():
                pygame.draw.rect(
                    Display, terrain_colors[(x, y)], (xx, yy, 3, 3))
            else:
                pygame.draw.rect(Display, oceanColor, (xx, yy, 3, 3))
    for ii, o in enumerate(options):
        i += blit_txt((">" if cursor == ii else " ") + o, 30,
                      i * 22 + 280, Display, maxlength=60)
    pygame.display.update()


if __name__ == '__main__':
    compile.main()
    for root, dirs, files in itertools.chain(os.walk("json_data"), os.walk("saves")):
        for file in files:
            if not file.endswith(".json"):
                continue
            with open(f"{root}/{file}") as f:
                j = json.loads(f.read())
                if j["file_type"] == "gen":
                    filenames.insert(0, f"{root}/{file}")
                    jsondata.insert(0, j)
                    options.insert(
                        0, f"{root}/{file}: generate a world with size {j['Config']['WORLD_SIZE']}")
                else:
                    filenames.append(f"{root}/{file}")
                    jsondata.append(j)
                    options.append(
                        f"load {j['Magic']['Material'][0]} {j['Magic']['Name']} in Y{j['current_year']}")
    while True:
        update_screen()
