from jinja2 import Environment, FileSystemLoader


def getinv(var, index):
    return f"({var}[{index}] if {index} in {var}.keys() else 0)"


def inv_opeq(var, index, op, val):
    return f"{var}[{index}] = ({var}[{index}] {op} {val} if {index} in {var}.keys() else (0 {op} 1) * {val})"


def resistance_add(a, b):
    return f"({a}) * ({b}) / ({a} + {b})"


def extract_material(t):
    return f"({t}[5:] if {t}[:5] == 'Tame ' else {t}[4:] if {t}[:4] == 'Cut ' else {t}[:-6] if {t}[-6:] == ' Goods' else {t})"


def add_inv(a, b):
    return f"{'{'}k: ({a}[k] if k in {a}.keys() else 0) + ({b}[k] if k in {b}.keys() else 0) for k in [*{a}.keys(), *{b}.keys()] {'}'}"


def cultural_distance(a, b):
    return f"sum(({a}.cultural_values[k] - {b}.cultural_values[k]) ** 2 for k in {a}.cultural_values.keys())"


def str_list(a):
    return f"({a}[0] if len({a}) == 1 else {a}[0] + ' and ' + {a}[1] if len({a}) == 2 else (', '.join({a}[:-1]) + ', and ' + {a}[-1])).lower()"


def main():
    environment = Environment(loader=FileSystemLoader(""))
    results_template = environment.get_template("sim_template.py")
    context = {
        "getinv": getinv,
        "inv_opeq": inv_opeq,
        "resistance_add": resistance_add,
        "extract_material": extract_material,
        "add_inv": add_inv,
        "cultural_distance": cultural_distance,
        "str_list": str_list
    }

    with open('sim.py', mode="w") as results:
        results.write(results_template.render(context))


if __name__ == "__main__":
    main()
