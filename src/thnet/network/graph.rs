use crate::thnet::network::Pipe;

/// Estructura para el manejo de la matriz de incidencia de la red.
#[derive(Debug, Clone, Default)]
pub struct IncidenceMatrix {
    /// Coeficientes de la matriz: de tamaño (n_nodes, n_pipes).
    /// matriz[i][j] = -1 si la rama j sale de i, +1 si entra a i, 0 si no conecta.
    pub data: Vec<Vec<f64>>,
}

impl IncidenceMatrix {
    /// Construye la matriz de incidencia a partir del número de nodos y las ramas de la red.
    pub fn build(n_nodes: usize, pipes: &[Pipe]) -> Self {
        let n_pipes = pipes.len();
        let mut data = vec![vec![0.0; n_pipes]; n_nodes];
        for (j, pipe) in pipes.iter().enumerate() {
            if pipe.node_up < n_nodes {
                data[pipe.node_up][j] = -1.0;
            }
            if pipe.node_dn < n_nodes {
                data[pipe.node_dn][j] = 1.0;
            }
        }
        IncidenceMatrix { data }
    }
}
