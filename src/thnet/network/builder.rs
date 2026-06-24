use crate::thnet::network::{Network, Node, Pipe, OpenTank, ClosedTank, StratifiedTank, HeatExchanger, NodeId, PipeId};

/// Constructor declarativo y fluido de la red.
#[derive(Debug, Default)]
pub struct NetworkBuilder {
    network: Network,
}

impl NetworkBuilder {
    /// Crea un nuevo constructor de red vacío.
    pub fn new() -> Self {
        NetworkBuilder {
            network: Network::new(),
        }
    }

    /// Agrega un nodo a la red consumiendo y retornando el builder.
    pub fn add_node(mut self, node: Node) -> Self {
        self.network.add_node(node);
        self
    }

    /// Agrega un nodo a la red, devolviendo el ID del nodo y el builder.
    pub fn add_node_with_id(mut self, node: Node) -> (Self, NodeId) {
        let id = self.network.add_node(node);
        (self, id)
    }

    /// Agrega una tubería a la red consumiendo y retornando el builder.
    pub fn add_pipe(mut self, pipe: Pipe) -> Self {
        self.network.add_pipe(pipe);
        self
    }

    /// Agrega una tubería a la red, devolviendo el ID de la tubería y el builder.
    pub fn add_pipe_with_id(mut self, pipe: Pipe) -> (Self, PipeId) {
        let id = self.network.add_pipe(pipe);
        (self, id)
    }

    /// Agrega un tanque abierto a la red.
    pub fn add_open_tank(mut self, tank: OpenTank) -> Self {
        self.network.add_open_tank(tank);
        self
    }

    /// Agrega un tanque cerrado a la red.
    pub fn add_closed_tank(mut self, tank: ClosedTank) -> Self {
        self.network.add_closed_tank(tank);
        self
    }

    /// Agrega un tanque de estratificación a la red.
    pub fn add_stratified_tank(mut self, tank: StratifiedTank) -> Self {
        self.network.add_stratified_tank(tank);
        self
    }

    /// Agrega un intercambiador de calor a la red.
    pub fn add_heat_exchanger(mut self, hx: HeatExchanger) -> Self {
        self.network.add_heat_exchanger(hx);
        self
    }

    /// Retorna la red construida.
    pub fn build(self) -> Network {
        self.network
    }
}
