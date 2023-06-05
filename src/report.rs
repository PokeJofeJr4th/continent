use crate::{usize_to_vec, Terrain, World, Snapshot};

fn range2d(range: [usize; 4]) -> impl Iterator<Item = (usize, usize)> {
    (range[0]..range[2]).flat_map(move |x| (range[1]..range[3]).map(move |y: usize| (x, y)))
}

const REPORT_STYLE: &str = "
            .tooltip {
                position: relative;
                display: inline-block;
                border-bottom: 1px dotted black;
            }

            .tooltip .tooltiptext {
                visibility: hidden;
                bottom: 100%;
                left: 50%;
                margin-left: -20px;
                background-color: black;
                color: #fff;
                text-align: center;
                padding: 5px;
                border-radius: 6px;
                position: absolute;
                z-index: 1;
            }

            .tooltip:hover .tooltiptext {
                visibility:visible;
            }

            .tradeleft {
                background: linear-gradient(to top left,
                    rgba(255,255,255,0) 0%,
                    rgba(255,255,255,0) calc(50% - 2px),
                    rgba(255,255,255,1) 50%,
                    rgba(255,255,255,0) calc(50% + 2px),
                    rgba(255,255,255,0) 100%
                );
            }
            .traderight {
                background: linear-gradient(to top right,
                    rgba(255,255,255,0) 0%,
                    rgba(255,255,255,0) calc(50% - 2px),
                    rgba(255,255,255,1) 50%,
                    rgba(255,255,255,0) calc(50% + 2px),
                    rgba(255,255,255,0) 100%
                );
            }
            .tradevertical {
                background: linear-gradient(to right,
                    rgba(255,255,255,0) 0%,
                    rgba(255,255,255,0) calc(50% - 2px),
                    rgba(255,255,255,1) 50%,
                    rgba(255,255,255,0) calc(50% + 2px),
                    rgba(255,255,255,0) 100%
                );
            }
            .tradehorizontal {
                background: linear-gradient(to top,
                    rgba(255,255,255,0) 0%,
                    rgba(255,255,255,0) calc(50% - 2px),
                    rgba(255,255,255,1) 50%,
                    rgba(255,255,255,0) calc(50% + 2px),
                    rgba(255,255,255,0) 100%
                );
            }
            .small_chart {
                display: inline-block;
                width: 30%;
            }";

fn get_trade_connections(world: &World, range: [usize; 4], scale: usize) -> String {
    world
        .trade_connections_list
        .iter()
        .map(|(first, second)| {
            let first = usize_to_vec(*first, &world.config);
            let second = usize_to_vec(*second, &world.config);
            let dy = second[1] as i32 - first[1] as i32;
            let dx = second[0] as i32 - first[0] as i32;
            let class_name = {
                if dx == 0 {
                    "tradevertical"
                } else if dy == 0 {
                    "tradehorizontal"
                } else if dx * dy > 0 {
                    "traderight"
                } else {
                    "tradeleft"
                }
            };
            let left = ((first[0].min(second[0]) - range[0]) * 20 + 15) / scale;
            let top = ((first[1].min(second[1]) - range[1]) * 20 + 15) / scale;
            let w = (dx.abs() * 20) / scale as i32 + 4;
            let h = (dy.abs() * 20) / scale as i32 + 4;
            format!(
                "<span class=\"{class_name}\"
                        style=\"
                            position:absolute;
                            left: {left}px;
                            top: {top}px;
                            width: {w}px;
                            height: {h}px;
                        \"></span>"
            )
        })
        .collect()
}

fn map(world: &World, range: [usize; 4], scale: usize, include_trade: bool) -> String {
    let s = 20 / scale;
    let width = (range[2] - range[0]) / scale;
    let height = (range[3] - range[1]) / scale;
    let squares: String = range2d(range)
        .map(|(x, y)| {
            let idx = x + y * world.config.world_size.0;
            let color = match world.region_list[world.region_map[idx]].terrain {
                Terrain::Ocean => "#008",
                Terrain::Plain => "#080",
                Terrain::Forest => "#084",
                Terrain::Mountain => "#444",
                Terrain::Desert => "#880",
                Terrain::Jungle => "#480",
            };
            format!(
                "<span style=\"width:{s}px;
                    height:{s}px;
                    background-color: {color};
                    position:absolute;
                    left:{left}px;
                    top:{top}px;\"></span>",
                left = ((x - range[0]) * 20 + 3) / scale + 2,
                top = ((y - range[1]) * 20 + 3) / scale + 2,
            )
        })
        .collect();
    let trade_routes = if include_trade {
        get_trade_connections(world, range, scale)
    } else {
        String::new()
    };
    let cities: String = range2d(range)
        .map(|(x, y)| x + y * world.config.world_size.0)
        .filter_map(|idx| world.city_list.get(&idx))
        .map(|city| {
            let [x, y] = usize_to_vec(city.pos, &world.config)[..] else {return String::new()};
            format!(
                "<a href=\"#city_({x}, {y})\" class=\"tooltip\" style=\"
                border-radius: {border_radius}px;
                position:absolute;
                width:{size}px;
                height:{size}px;
                background-color:white;
                border:{border_width}px solid black;
                left:{left}px;
                top:{top}px;\">
                    <span class=\"tooltiptext\">{name}</span>
                </a>",
                border_radius = 10 / scale,
                size = 8 / scale,
                border_width = 4 / scale,
                left = ((x - range[0]) * 20 + 5) / scale + 2,
                top = ((y - range[1]) * 20 + 5) / scale + 2,
                name = city.name,
            )
        })
        .collect();
    format!(
        "<div style=\"
            width:{w}px;
            height:{h}px;
            position:relative;
            background-color:black;
            padding:5px\">
            {squares}
            {trade_routes}
            {cities}
        </div>",
        w = width * 20,
        h = height * 20,
    )
}

pub fn chart_script(world: &World) -> String {
    let draw_chart: String = world
        .city_list
        .iter()
        .map(|(pos, city)| 
        {
            let snapshots: Vec<(String, &Snapshot)> = (0..).map_while(|n| {
                let idx: String = (n*100).to_string();
                city.data.get(&idx).map(|snapshot| (idx, snapshot))}).collect();
            let pop_data: String = snapshots.iter().map(|(year, snapshot)| 
                format!(",['{year}', {population}]", population = snapshot.population)
            ).collect();
            let item_keys = &city.data.get("100").unwrap().imports;
            let import_data: String = snapshots.iter().map(|(year, snapshot)|
        {
            String::new()
        }
            ).collect();
            let [x, y] = usize_to_vec(*pos, &world.config)[..] else { return String::new() };
            format!(
            "var pop_data = google.visualization.arrayToDataTable([['Year', 'Population']{pop_data}]);
            
            var pop_options = {{'title':'City Population'}};
            
            var pop_chart = new google.visualization.LineChart(document.getElementById('popchart_({x}, {y})'));
            pop_chart.draw(pop_data, pop_options);"
            )
        })
        .collect();
    format!(
        "google.charts.load('current', {{'packages':['corechart']}});
    google.charts.setOnLoadCallback(drawChart);
    function drawChart() {{
        {draw_chart}
    }}"
    )
}

pub fn report(world: &World) -> String {
    let mainmap = map(
        world,
        [0, 0, world.config.world_size.0, world.config.world_size.1],
        1,
        true,
    );
    let magic = String::new();
    let cities: String = world
        .city_list
        .iter()
        .map(|(pos, city)| {
            let [x, y] = usize_to_vec(*pos, &world.config)[..] else {return String::new()};
            format!(
                "<h3 id=\"city_({x}, {y})\">{name}</h3>
                    {map}
                    <div class=\"small_chart\" id=\"popchart_({x}, {y})\"></div>
                    <div class=\"small_chart\" id=\"importchart_({x}, {y})\"></div>
                    <div class=\"small_chart\" id=\"prodchart_({x}, {y})\"></div>",
                name = city.name,
                map = map(
                    world,
                    [
                        x.max(5) - 5,
                        y.max(5) - 5,
                        x.min(world.config.world_size.0 - 6) + 6,
                        y.min(world.config.world_size.1 - 6) + 6
                    ],
                    2,
                    false
                )
            )
        })
        .collect();
    let resources = String::new();
    let regions = String::new();
    format!(
        "<!DOCTYPE html>
    <html lang=\"en\">
    <head>
        <meta charset=\"UTF-8\">
        <title>World Report</title>
        <style>
        {REPORT_STYLE}
        </style>
        <script type=\"text/javascript\" src=\"https://www.gstatic.com/charts/loader.js\"></script>
        <script type=\"text/javascript\">{script}</script>
    </head>
    <body>
        <h1>World Report</h1>
        <h2>Contents</h2>
        <ul>
            <li><a href=\"#h2_Magic\">Magic</a></li>
            <li><a href=\"#h2_Cities\">Cities</a></li>
            <li><a href=\"#h2_Resources\">Resources</a></li>
            <li><a href=\"#h2_Regions\">Regions</a></li>
        </ul>
        {mainmap}
        <h2 id=\"h2_Magic\">Magic</h2>
        {magic}
        <h2 id=\"h2_Cities\">Cities</h2>
        {cities}
        <h2 id=\"h2_Resources\">Resources</h2>
        {resources}
        <h2 id=\"h2_Regions\">Regions</h2>
        {regions}
    </body>
    </html>",
        script = chart_script(world)
    )
}
