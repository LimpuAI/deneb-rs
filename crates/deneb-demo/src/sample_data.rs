//! 演示数据
//!
//! 为 4 种图表类型提供硬编码的 CSV 数据。

/// 折线图演示数据（时间序列）
pub fn line_chart_csv() -> &'static str {
    "x,y\n0,10\n1,25\n2,18\n3,32\n4,28\n5,45\n6,38\n7,52\n8,48\n9,55\n10,42\n11,60\n12,35\n13,50\n14,65\n15,58\n16,70\n17,45\n18,62\n19,75"
}

/// 柱状图演示数据（分类数据）
pub fn bar_chart_csv() -> &'static str {
    "category,value\nElectronics,450\nClothing,320\nFood,280\nBooks,190\nSports,350\nMusic,210"
}

/// 散点图演示数据（两组聚类）
pub fn scatter_chart_csv() -> &'static str {
    "x,y,group\n1.2,3.4,A\n1.8,4.1,A\n2.1,3.8,A\n1.5,4.5,A\n2.5,3.2,A\n1.9,3.9,A\n2.3,4.3,A\n1.7,3.6,A\n5.5,7.2,B\n6.1,6.8,B\n5.8,7.5,B\n6.5,7.1,B\n5.2,6.5,B\n6.8,7.8,B\n5.9,6.9,B\n6.3,7.4,B\n3.5,5.2,A\n3.8,5.8,B\n2.8,4.2,A\n4.2,6.1,B"
}

/// 面积图演示数据（2 系列）
pub fn area_chart_csv() -> &'static str {
    "x,y1,y2\n0,20,10\n1,35,15\n2,28,22\n3,45,30\n4,38,25\n5,55,35\n6,48,40\n7,65,45\n8,58,38\n9,72,50\n10,62,42\n11,78,55"
}
