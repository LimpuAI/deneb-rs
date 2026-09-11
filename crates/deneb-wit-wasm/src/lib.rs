//! deneb-wit-wasm: WASI Component Model 导出层(v2 — resource session 协议)
//!
//! 使用 wit-bindgen 0.57 从 deneb-viz.wit 生成 guest 绑定,
//! 将 deneb-wit 的功能导出为标准 WASI Component。
//!
//! v2:`chart` resource 有状态会话(constructor/update-data/resize/set-state/
//! set-theme/render(t)/hit-regions),取代 v1 的自由函数 render/hit-test。
//! Arrow/Parquet 解析通过导入 limpuai:data 解析器组件实现委托。

wit_bindgen::generate!({
    world: "deneb:viz/deneb-viz@4.0.0",
    path: "../deneb-wit/wit",
    generate_all,
});

use deneb_wit::lib_mode;
use deneb_wit::session::ChartSession;
use deneb_wit::wit_types::*;

use exports::deneb::viz::chart_renderer::GuestChart;
use exports::deneb::viz::data_parser::Guest as DataParserGuest;

mod convert;
use convert::*;

struct DenebVizComponent;

impl DataParserGuest for DenebVizComponent {
    fn parse_csv(
        data: Vec<u8>,
    ) -> Result<exports::deneb::viz::data_parser::DataTable, String> {
        let wit = lib_mode::parse_data(&data, "csv")?;
        Ok(wit_data_table_to_bindgen(wit))
    }

    fn parse_json(
        data: Vec<u8>,
    ) -> Result<exports::deneb::viz::data_parser::DataTable, String> {
        let wit = lib_mode::parse_data(&data, "json")?;
        Ok(wit_data_table_to_bindgen(wit))
    }

    fn parse_arrow(
        data: Vec<u8>,
    ) -> Result<exports::deneb::viz::data_parser::DataTable, String> {
        let dt = limpuai::data::arrow_parser::parse(&data)?;
        Ok(limpuai_dt_to_bindgen(dt))
    }

    fn parse_parquet(
        data: Vec<u8>,
    ) -> Result<exports::deneb::viz::data_parser::DataTable, String> {
        let dt = limpuai::data::parquet_parser::parse(&data)?;
        Ok(limpuai_dt_to_bindgen(dt))
    }
}

// chart-renderer 接口仅含 resource — 接口级 Guest 为空标记,
// resource 本体在 GuestChart(constructor + 六方法,单 trait)。

/// `chart` resource 实现体 — 持有 deneb-wit ChartSession
///
/// trait 方法为 &self(bindgen 约定),会话可变性经 RefCell。
pub struct ChartResource {
    session: std::cell::RefCell<ChartSession>,
}

impl GuestChart for ChartResource {
    fn new(
        spec: exports::deneb::viz::chart_renderer::ChartSpec,
        theme: Option<exports::deneb::viz::chart_renderer::Theme>,
    ) -> Self {
        let wit_spec = bindgen_to_wit_chart_spec(spec);
        let wit_theme = theme.map(bindgen_to_wit_theme);
        match ChartSession::new(wit_spec, wit_theme) {
            Ok(session) => Self { session: std::cell::RefCell::new(session) },
            Err(e) => {
                // constructor 无错误通道:以最小会话降级,后续 update-data 前 render 报错
                let fallback = WitChartSpec {
                    mark: "bar".into(),
                    x_field: "x".into(),
                    y_field: "y".into(),
                    color_field: None,
                    open_field: None,
                    high_field: None,
                    low_field: None,
                    close_field: None,
                    theta_field: None,
                    size_field: None,
                    width: 1.0,
                    height: 1.0,
                    title: None,
                    animation: None,
                };
                match ChartSession::new(fallback, None) {
                    Ok(session) => Self { session: std::cell::RefCell::new(session) },
                    Err(_) => panic!("chart constructor failed: {}", e),
                }
            }
        }
    }

    fn update_data(&self, data: Vec<u8>, format: String) -> Result<(), String> {
        match format.as_str() {
            "arrow" => {
                let dt = limpuai::data::arrow_parser::parse(&data)?;
                let wit = limpuai_dt_to_wit(dt);
                let table = deneb_wit::convert::wit_data_table_to_data_table(wit)
                    .map_err(|e| e.to_string())?;
                self.session.borrow_mut().set_table(table)
            }
            "parquet" => {
                let dt = limpuai::data::parquet_parser::parse(&data)?;
                let wit = limpuai_dt_to_wit(dt);
                let table = deneb_wit::convert::wit_data_table_to_data_table(wit)
                    .map_err(|e| e.to_string())?;
                self.session.borrow_mut().set_table(table)
            }
            _ => self.session.borrow_mut().update_data(&data, &format),
        }
    }

    fn resize(&self, width: f64, height: f64) {
        self.session.borrow_mut().resize(width, height);
    }

    fn set_state(&self, state: exports::deneb::viz::chart_renderer::InteractionState) {
        self.session.borrow_mut().set_state(WitInteractionState {
            hovered: state.hovered,
            selected: state.selected,
        });
    }

    fn set_theme(&self, theme: exports::deneb::viz::chart_renderer::Theme) {
        self.session.borrow_mut().set_theme(bindgen_to_wit_theme(theme));
    }

    fn render(
        &self,
        t: f64,
    ) -> Result<exports::deneb::viz::chart_renderer::RenderResult, String> {
        let wit = self.session.borrow_mut().render(t)?;
        Ok(wit_render_result_to_bindgen(wit))
    }

    fn hit_regions(&self) -> Vec<exports::deneb::viz::chart_renderer::HitRegion> {
        self.session
            .borrow()
            .hit_regions()
            .into_iter()
            .map(wit_hit_region_to_bindgen)
            .collect()
    }
}

impl exports::deneb::viz::chart_renderer::Guest for DenebVizComponent {
    type Chart = ChartResource;
}

export!(DenebVizComponent);
