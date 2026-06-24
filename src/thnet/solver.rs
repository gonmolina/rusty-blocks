/// Solvedor termohidráulico de red completa (THNet Solver).
///
/// ## Algoritmo
///
/// En cada paso de tiempo Δt se ejecutan dos fases en secuencia
/// (fraccionamiento de operadores / Operator Splitting):
///
/// **Fase 1 — Hidráulica implícita (Newton-Raphson)**
///
/// Se ensambla y resuelve el sistema de conductancia nodal:
/// ```text
///   C · P^{k+1} = b
/// ```
/// donde C es la matriz de conductancias (análogo a la matriz nodal de SPICE),
/// P es el vector de presiones en nodos libres, y b contiene las fuentes.
///
/// Este enfoque es **incondicionalmente estable** para la dinámica hidráulica:
/// elimina la restricción CFL acústica (Δt << L/(N·c_sonido)) del enfoque
/// explícito anterior, permitiendo pasos de tiempo de 1–10 s.
///
/// **Fase 2 — Transporte térmico implícito upwind**
///
/// Con los caudales W^{k+1} conocidos se actualiza la entalpía en cada celda
/// de las tuberías usando un esquema upwind implícito en espacio (space-marching).
/// Este esquema es incondicionalmente estable para cualquier número de Courant.
///
/// ## Analogía eléctrica (MNA / SPICE)
///
/// | Cantidad eléctrica | Cantidad hidráulica |
/// |---|---|
/// | Tensión V | Presión P [Pa] |
/// | Corriente I | Caudal W [kg/s] |
/// | Conductancia 1/R | G_j = Δt/(I_j + Δt·R_lin_j) |
/// | Fuente de corriente | S_j (inercia + gravedad + bomba) |
/// | Nodo GND | Nodo Dirichlet (presión fija) |

use super::network::Network;
use super::thermo;

// ─────────────────────────────────────────────────────────────────────────────
// SOLVEDOR
// ─────────────────────────────────────────────────────────────────────────────

/// Tipo de solver lineal a utilizar para resolver la hidráulica de la red.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LinearSolverKind {
    /// Eliminación gaussiana densa (por defecto para redes pequeñas/medianas)
    #[default]
    GaussianElimination,
    /// Método iterativo Gauss-Seidel con Relajación Sucesiva (SOR)
    GaussSeidelSor,
}

/// Solvedor acoplado hidráulico-térmico para redes de agua monofásica.
pub struct Solver {
    /// Máximo de iteraciones Newton-Raphson por paso hidráulico
    pub max_newton_iter: usize,
    /// Tolerancia de convergencia en caudal [kg/s]
    pub tol_flow: f64,
    /// Tiempo de simulación acumulado [s]
    pub time: f64,
    /// Tipo de solver lineal a utilizar
    pub linear_solver: LinearSolverKind,
    /// Factor de relajación (omega) para SOR (típicamente entre 1.0 y 1.9, default: 1.5)
    pub sor_omega: f64,
    /// Tolerancia de convergencia para el solver iterativo (default: 1e-6 Pa)
    pub sor_tol: f64,
    /// Cantidad máxima de iteraciones para el solver iterativo (default: 1000)
    pub sor_max_iter: usize,
}

impl Solver {
    pub fn new() -> Self {
        Solver {
            max_newton_iter: 50,
            tol_flow: 1e-9,
            time: 0.0,
            linear_solver: LinearSolverKind::GaussianElimination,
            sor_omega: 1.5,
            sor_tol: 1e-6,
            sor_max_iter: 1000,
        }
    }

    /// Avanza un paso de tiempo Δt [s].
    ///
    /// Ejecuta hidráulica (implícita) → térmica (upwind implícita).
    pub fn step(&mut self, net: &mut Network, dt: f64) {
        self.update_check_valves(net);
        self.update_prvs(net);
        self.update_open_tanks_pressures(net);
        self.update_closed_tanks_pressures(net);
        self.update_stratified_tanks_pressures(net);
        self.step_hydraulic(net, dt);
        self.step_thermal(net, dt);
        self.update_open_tanks_levels(net, dt);
        self.update_closed_tanks_levels(net, dt);
        self.update_stratified_tanks_levels_and_temp(net, dt);
        self.time += dt;
    }

    /// Actualiza el estado de las check valves basándose en el caudal del paso anterior y aplicando histéresis.
    fn update_check_valves(&self, net: &mut Network) {
        const W_THRESHOLD: f64 = 1e-4;
        for pipe in &mut net.pipes {
            if pipe.is_check_valve {
                if pipe.check_valve_closed {
                    if pipe.flow > W_THRESHOLD {
                        pipe.check_valve_closed = false;
                    }
                } else {
                    if pipe.flow < -W_THRESHOLD {
                        pipe.check_valve_closed = true;
                    }
                }
            }
        }
    }

    /// Actualiza el estado de las válvulas de alivio de presión (PRV) basándose en la presión del nodo aguas arriba.
    fn update_prvs(&self, net: &mut Network) {
        for pipe in &mut net.pipes {
            if let super::network::BranchComponent::Prv(ref mut prv) = pipe.component {
                let p_up = net.nodes[pipe.node_up].pressure;
                if prv.is_open {
                    if p_up < prv.set_pressure - prv.blowdown {
                        prv.is_open = false;
                    }
                } else {
                    if p_up > prv.set_pressure {
                        prv.is_open = true;
                    }
                }
            }
        }
    }

    /// Actualiza la presión y condición de Dirichlet de los nodos asociados a tanques abiertos.
    fn update_open_tanks_pressures(&self, net: &mut Network) {
        const G: f64 = 9.806_65;
        for tank in &net.open_tanks {
            let rho = net.nodes[tank.node_id].density();
            let p = tank.p_atm + rho * G * tank.level;
            let node = &mut net.nodes[tank.node_id];
            node.pressure = p;
            node.fixed_pressure = Some(p);
        }
    }

    /// Actualiza el nivel y la presión resultante de los tanques abiertos tras el paso temporal.
    fn update_open_tanks_levels(&self, net: &mut Network, dt: f64) {
        const G: f64 = 9.806_65;
        let mut w_nets = vec![0.0_f64; net.open_tanks.len()];
        for (i, tank) in net.open_tanks.iter().enumerate() {
            let node_id = tank.node_id;
            let mut w_net = 0.0_f64;
            for pipe in &net.pipes {
                if pipe.node_dn == node_id {
                    w_net += pipe.flow;
                }
                if pipe.node_up == node_id {
                    w_net -= pipe.flow;
                }
            }
            w_nets[i] = w_net;
        }

        for (i, tank) in net.open_tanks.iter_mut().enumerate() {
            let node_id = tank.node_id;
            let rho = net.nodes[node_id].density().max(1.0);
            let w_net = w_nets[i];

            tank.level += dt * w_net / (rho * tank.area);
            tank.level = tank.level.clamp(tank.level_min, tank.level_max);

            let p = tank.p_atm + rho * G * tank.level;
            let node = &mut net.nodes[node_id];
            node.pressure = p;
            node.fixed_pressure = Some(p);
        }
    }

    /// Actualiza la presión y condición de Dirichlet de los nodos asociados a tanques cerrados.
    fn update_closed_tanks_pressures(&self, net: &mut Network) {
        for tank in &net.closed_tanks {
            let rho = net.nodes[tank.node_id].density();
            let p = tank.calculate_pressure(rho);
            let node = &mut net.nodes[tank.node_id];
            node.pressure = p;
            node.fixed_pressure = Some(p);
        }
    }

    /// Actualiza el nivel y la presión resultante de los tanques cerrados tras el paso temporal.
    fn update_closed_tanks_levels(&self, net: &mut Network, dt: f64) {
        let mut w_nets = vec![0.0_f64; net.closed_tanks.len()];
        for (i, tank) in net.closed_tanks.iter().enumerate() {
            let node_id = tank.node_id;
            let mut w_net = 0.0_f64;
            for pipe in &net.pipes {
                if pipe.node_dn == node_id {
                    w_net += pipe.flow;
                }
                if pipe.node_up == node_id {
                    w_net -= pipe.flow;
                }
            }
            w_nets[i] = w_net;
        }

        for (i, tank) in net.closed_tanks.iter_mut().enumerate() {
            let node_id = tank.node_id;
            let rho = net.nodes[node_id].density().max(1.0);
            let w_net = w_nets[i];

            tank.level += dt * w_net / (rho * tank.area);
            tank.level = tank.level.clamp(tank.level_min, tank.level_max);

            let p = tank.calculate_pressure(rho);
            let node = &mut net.nodes[node_id];
            node.pressure = p;
            node.fixed_pressure = Some(p);
        }
    }

    /// Actualiza la presión, temperatura y entalpía del nodo asociado a tanques estratificados.
    fn update_stratified_tanks_pressures(&self, net: &mut Network) {
        for tank in &net.stratified_tanks {
            let p = tank.calculate_pressure();
            let t_bottom = tank.layer_temp()[0];
            let node = &mut net.nodes[tank.node_id()];
            node.pressure = p;
            node.fixed_pressure = Some(p);
            node.temperature = t_bottom;
            node.h = thermo::enthalpy(t_bottom);
        }
    }

    /// Actualiza el nivel y la estratificación térmica en los tanques estratificados tras el paso temporal.
    fn update_stratified_tanks_levels_and_temp(&self, net: &mut Network, dt: f64) {
        let mut w_nets = vec![0.0_f64; net.stratified_tanks.len()];
        for (i, tank) in net.stratified_tanks.iter().enumerate() {
            let node_id = tank.node_id();
            let mut w_net = 0.0_f64;
            for pipe in &net.pipes {
                if pipe.node_dn == node_id {
                    w_net += pipe.flow;
                }
                if pipe.node_up == node_id {
                    w_net -= pipe.flow;
                }
            }
            w_nets[i] = w_net;
        }

        for (i, tank) in net.stratified_tanks.iter_mut().enumerate() {
            let node_id = tank.node_id();
            tank.update_levels_and_temp(w_nets[i], dt);

            // Forzar el nodo a la nueva presión y temperatura
            let p_new = tank.calculate_pressure();
            let t_bot_new = tank.layer_temp()[0];
            let node = &mut net.nodes[node_id];
            node.pressure = p_new;
            node.fixed_pressure = Some(p_new);
            node.temperature = t_bot_new;
            node.h = thermo::enthalpy(t_bot_new);
        }
    }

    // ─────────────────────────────────────────────────────────────────────
    // FASE 1: HIDRÁULICA IMPLÍCITA
    // ─────────────────────────────────────────────────────────────────────

    fn step_hydraulic(&self, net: &mut Network, dt: f64) {
        // Validar conectividad (detector de islas/nodos desconectados)
        let isolated = self.find_isolated_nodes(net);
        if !isolated.is_empty() {
            panic!(
                "Error de topología de red: Nodos aislados detectados: {:?}. \
                 Estos nodos no tienen un camino activo hacia ningún nodo con presión fija (referencia Dirichlet). \
                 Verifique que todas las válvulas estén abiertas o que la red esté correctamente conectada.",
                isolated
            );
        }

        let _n_nodes = net.n_nodes();

        // Mapa node_id → índice en el sistema libre (None si es Dirichlet)
        let free_map = build_free_map(&net.nodes);
        let n_free: usize = free_map.iter().filter(|x| x.is_some()).count();

        if n_free == 0 {
            // Todos los nodos tienen presión fija: calcular caudales directamente
            // (sin sistema lineal — una iteración Newton es suficiente)
            self.update_flows_fixed_pressures(net, dt);
            return;
        }

        // Iteraciones Newton-Raphson sobre la resistencia linealizada
        for _iter in 0..self.max_newton_iter {
            let n_pipes = net.n_pipes();

            // ── Calcular G_j y S_j para cada tubería ──────────────────────
            // G_j = Δt / (I_j + Δt·R_lin_j)          [conductancia]
            // S_j = (I_j·W_j^k + Δt·ΔP_grav_j) / (I_j + Δt·R_lin_j)  [fuente]
            let mut gj = vec![0.0_f64; n_pipes];
            let mut sj = vec![0.0_f64; n_pipes];

            for (j, pipe) in net.pipes.iter().enumerate() {
                let rho = pipe.mean_density();
                let inertia = pipe.hydraulic_inertia(rho); // I = ρ·L/A
                let r_lin = pipe.resistance_linearized();   // R_lin = K·|W|/ρ
                let dp_grav = pipe.gravity_pressure_drop(); // -ρ·g·Δz
                let dp_pump = pipe.pump_pressure_src();

                let denom = (inertia + dt * r_lin).max(1e-30);
                gj[j] = dt / denom;
                sj[j] = (inertia * pipe.flow + dt * (dp_grav + dp_pump)) / denom;
            }

            // ── Ensamblar matriz de conductancia C y vector RHS b ─────────
            //
            // Derivación (ver MATH_SOLVER.md §1.4):
            //   Para cada nodo libre i, balance de masa:
            //   Σ_{j: d_j=i} W_j  -  Σ_{j: u_j=i} W_j = 0
            //
            //   Sustituyendo W_j = G_j·(P_u_j - P_d_j) + S_j:
            //
            //   C[i,i]       += G_j  (diagonal, para cada tubería j que toca i)
            //   C[i, free(l)] -= G_j  (off-diagonal al otro extremo l, si libre)
            //   b[i]         += G_j·P_fixed_l  (si l es Dirichlet → va al RHS)
            //
            //   Signo de S_j en b[i]:
            //     +S_j si j llega a i (j.node_dn == i)
            //     -S_j si j sale de i (j.node_up == i)
            //
            let mut p_free = vec![0.0_f64; n_free];
            for (node_id, fi) in free_map.iter().enumerate() {
                if let Some(i) = fi {
                    p_free[*i] = net.nodes[node_id].pressure;
                }
            }

            match self.linear_solver {
                LinearSolverKind::GaussianElimination => {
                    let mut c_mat = vec![vec![0.0_f64; n_free]; n_free];
                    let mut b_vec = vec![0.0_f64; n_free];

                    for (j, pipe) in net.pipes.iter().enumerate() {
                        let u = pipe.node_up;
                        let d = pipe.node_dn;
                        let g = gj[j];
                        let s = sj[j];

                        if let Some(iu) = free_map[u] {
                            c_mat[iu][iu] += g;
                            b_vec[iu] -= s;
                            match free_map[d] {
                                Some(id) => {
                                    c_mat[iu][id] -= g;
                                }
                                None => {
                                    let p_fixed = net.nodes[d].fixed_pressure.unwrap();
                                    b_vec[iu] += g * p_fixed;
                                }
                            }
                        }

                        if let Some(id) = free_map[d] {
                            c_mat[id][id] += g;
                            b_vec[id] += s;
                            match free_map[u] {
                                Some(iu) => {
                                    c_mat[id][iu] -= g;
                                }
                                None => {
                                    let p_fixed = net.nodes[u].fixed_pressure.unwrap();
                                    b_vec[id] += g * p_fixed;
                                }
                            }
                        }
                    }

                    p_free = gaussian_elimination(c_mat, b_vec);
                }
                LinearSolverKind::GaussSeidelSor => {
                    let mut c_sparse = SparseMatrix::new(n_free);
                    let mut b_vec = vec![0.0_f64; n_free];

                    for (j, pipe) in net.pipes.iter().enumerate() {
                        let u = pipe.node_up;
                        let d = pipe.node_dn;
                        let g = gj[j];
                        let s = sj[j];

                        if let Some(iu) = free_map[u] {
                            c_sparse.add(iu, iu, g);
                            b_vec[iu] -= s;
                            match free_map[d] {
                                Some(id) => {
                                    c_sparse.add(iu, id, -g);
                                }
                                None => {
                                    let p_fixed = net.nodes[d].fixed_pressure.unwrap();
                                    b_vec[iu] += g * p_fixed;
                                }
                            }
                        }

                        if let Some(id) = free_map[d] {
                            c_sparse.add(id, id, g);
                            b_vec[id] += s;
                            match free_map[u] {
                                Some(iu) => {
                                    c_sparse.add(id, iu, -g);
                                }
                                None => {
                                    let p_fixed = net.nodes[u].fixed_pressure.unwrap();
                                    b_vec[id] += g * p_fixed;
                                }
                            }
                        }
                    }

                    gauss_seidel_sor(
                        &c_sparse,
                        &b_vec,
                        &mut p_free,
                        self.sor_omega,
                        self.sor_tol,
                        self.sor_max_iter,
                    );
                }
            }

            // ── Actualizar presiones en nodos libres ───────────────────────
            for (node_id, fi) in free_map.iter().enumerate() {
                if let Some(i) = fi {
                    net.nodes[node_id].pressure = p_free[*i];
                }
            }

            // ── Actualizar caudales: W_j = G_j·(P_u - P_d) + S_j ─────────
            let mut max_delta_w = 0.0_f64;
            for (j, pipe) in net.pipes.iter_mut().enumerate() {
                let p_u = net.nodes[pipe.node_up].pressure;
                let p_d = net.nodes[pipe.node_dn].pressure;
                let w_new = gj[j] * (p_u - p_d) + sj[j];
                max_delta_w = max_delta_w.max((w_new - pipe.flow).abs());
                pipe.flow = w_new;
            }

            if max_delta_w < self.tol_flow {
                break;
            }
        }
    }

    /// Caso especial: todos los nodos tienen presión fija.
    /// Integra la ecuación de momento directamente sin sistema lineal.
    fn update_flows_fixed_pressures(&self, net: &mut Network, dt: f64) {
        for pipe in net.pipes.iter_mut() {
            let rho = pipe.mean_density();
            let inertia = pipe.hydraulic_inertia(rho);
            let dp_grav = pipe.gravity_pressure_drop();
            let p_u = net.nodes[pipe.node_up].pressure;
            let p_d = net.nodes[pipe.node_dn].pressure;

            // Iteración de punto fijo con relajación para resolver la resistencia y bomba cuadráticas implícitas
            let mut flow_iter = pipe.flow;
            for _ in 0..20 {
                let original_flow = pipe.flow;
                pipe.flow = flow_iter;
                let r_lin = pipe.resistance_linearized();
                let dp_pump = pipe.pump_pressure_src();
                pipe.flow = original_flow;

                let force = p_u - p_d + dp_grav + dp_pump;
                let denom = (inertia + dt * r_lin).max(1e-30);
                let next_flow = (inertia * pipe.flow + dt * force) / denom;
                flow_iter = 0.5 * flow_iter + 0.5 * next_flow;
            }
            pipe.flow = flow_iter;
        }
    }

    // ─────────────────────────────────────────────────────────────────────
    // FASE 2: TRANSPORTE TÉRMICO (upwind implícito en espacio)
    // ─────────────────────────────────────────────────────────────────────

    fn step_thermal(&self, net: &mut Network, dt: f64) {
        // Reset dynamic heat exchanger inputs
        for pipe in &mut net.pipes {
            pipe.q_hx_external = 0.0;
        }

        // Calculate NTU-ε heat exchange for each HX
        for hx in &net.heat_exchangers {
            let w_h = net.pipes[hx.pipe_hot].flow;
            let u_h = net.pipes[hx.pipe_hot].node_up;
            let d_h = net.pipes[hx.pipe_hot].node_dn;
            let t_h_in = if w_h >= 0.0 { net.nodes[u_h].temperature } else { net.nodes[d_h].temperature };

            let w_c = net.pipes[hx.pipe_cold].flow;
            let u_c = net.pipes[hx.pipe_cold].node_up;
            let d_c = net.pipes[hx.pipe_cold].node_dn;
            let t_c_in = if w_c >= 0.0 { net.nodes[u_c].temperature } else { net.nodes[d_c].temperature };

            let cp_h = thermo::cp(t_h_in);
            let cp_c = thermo::cp(t_c_in);

            let c_h = w_h.abs() * cp_h;
            let c_c = w_c.abs() * cp_c;

            let c_min = c_h.min(c_c);
            let c_max = c_h.max(c_c);

            let q = if c_min > 1e-10 {
                let ntu = hx.ua / c_min;
                let c_r = c_min / c_max;
                let epsilon = if (1.0 - c_r).abs() < 1e-5 {
                    ntu / (1.0 + ntu)
                } else {
                    let exp_term = (-ntu * (1.0 - c_r)).exp();
                    (1.0 - exp_term) / (1.0 - c_r * exp_term)
                };
                epsilon * c_min * (t_h_in - t_c_in)
            } else {
                0.0
            };

            net.pipes[hx.pipe_hot].q_hx_external -= q;
            net.pipes[hx.pipe_cold].q_hx_external += q;
        }

        let n_nodes = net.n_nodes();
        // Flujo de entalpía entrante a cada nodo [W]
        let mut phi_in = vec![0.0_f64; n_nodes];
        // Caudal saliente de cada nodo [kg/s]
        let mut w_out_total = vec![0.0_f64; n_nodes];
        // Caudal entrante a cada nodo [kg/s]
        let mut w_in_total = vec![0.0_f64; n_nodes];
        // Calor total extraído/cedido por el HX de cada tubería en este paso [W]
        // (solo para diagnóstico; no afecta al cálculo)
        let mut _q_hx_total: f64;

        // ── Actualizar celdas de cada tubería y calcular flujos nodales ───
        for pipe in net.pipes.iter_mut() {
            let w = pipe.flow;
            let u = pipe.node_up;
            let d = pipe.node_dn;

            if pipe.n_cells() == 0 {
                if w >= 0.0 {
                    w_out_total[u] += w;
                    w_in_total[d] += w;
                    phi_in[d] += w * net.nodes[u].h;
                } else {
                    let abs_w = -w;
                    w_out_total[d] += abs_w;
                    w_in_total[u] += abs_w;
                    phi_in[u] += abs_w * net.nodes[d].h;
                }
                continue;
            }

            let a = pipe.area();
            let dz_cell = pipe.length() / pipe.n_cells() as f64;
            let vol_cell = a * dz_cell;

            // Modo de transferencia de calor:
            // - HX (ua_hx > 0): calor variable = UA_cell·(T_cool - T_cell), implícito
            // - Fuente constante: heat_total / n_cells, explícito en entalpía
            let is_hx = pipe.ua_hx() > 0.0;
            let ua_cell = if is_hx { pipe.ua_hx() / pipe.n_cells() as f64 } else { 0.0 };
            let h_coolant = if is_hx {
                thermo::enthalpy(pipe.t_coolant())
            } else {
                0.0
            };
            let q_const_per_cell = if is_hx { 0.0 } else { (pipe.heat_total() + pipe.q_hx_external) / pipe.n_cells() as f64 };

            _q_hx_total = 0.0;

            if w >= 0.0 {
                // ── Flujo positivo: u → d ──────────────────────────────────
                //
                // Esquema upwind implícito en espacio (space-marching 0→N-1).
                //
                // Ambos son incondicionalmente estables para cualquier CFL y NTU.
                let h_in_boundary = net.nodes[u].h;
                let mut h_in = h_in_boundary;

                for i in 0..pipe.n_cells() {
                    let rho_cell = thermo::density(pipe.cell_temp[i]);
                    let m_cell = (rho_cell * vol_cell).max(1e-10);
                    let h_cell = thermo::enthalpy(pipe.cell_temp[i]);
                    let cfl = w * dt / m_cell;

                    let h_new = if pipe.wall_mass() > 0.0 {
                        let cp_cell = thermo::cp(pipe.cell_temp[i]);
                        let wall_mass_cell = pipe.wall_mass() / pipe.n_cells() as f64;
                        let wall_cp = pipe.wall_cp();
                        let ua_wall_cell = pipe.wall_ua() / pipe.n_cells() as f64;
                        let q_ext_cell = (pipe.heat_total() + pipe.q_hx_external) / pipe.n_cells() as f64;

                        let k_wall = (wall_mass_cell * wall_cp) / dt;
                        let d_wall = k_wall + ua_wall_cell;
                        let c_eff = (ua_wall_cell * k_wall) / d_wall;
                        let q_eff = (ua_wall_cell / d_wall) * q_ext_cell;

                        let ntu_w = c_eff * dt / (m_cell * cp_cell);
                        let h_wall = thermo::enthalpy(pipe.wall_temp[i]);

                        (h_cell + cfl * h_in + ntu_w * h_wall + q_eff * dt / m_cell) / (1.0 + cfl + ntu_w)
                    } else if is_hx {
                        let cp_cell = thermo::cp(pipe.cell_temp[i]);
                        // NTU en espacio de entalpía: adim., = UA·dt/(M·cp)
                        let ntu_h = ua_cell * dt / (m_cell * cp_cell);
                        (h_cell + cfl * h_in + ntu_h * h_coolant) / (1.0 + cfl + ntu_h)
                    } else {
                        (h_cell + cfl * h_in + q_const_per_cell * dt / m_cell) / (1.0 + cfl)
                    };

                    let t_fluid_new = thermo::temperature_from_enthalpy(h_new);
                    _q_hx_total += ua_cell * (t_fluid_new - pipe.t_coolant());
                    pipe.cell_temp[i] = t_fluid_new;

                    if pipe.wall_mass() > 0.0 {
                        let wall_mass_cell = pipe.wall_mass() / pipe.n_cells() as f64;
                        let wall_cp = pipe.wall_cp();
                        let ua_wall_cell = pipe.wall_ua() / pipe.n_cells() as f64;
                        let q_ext_cell = (pipe.heat_total() + pipe.q_hx_external) / pipe.n_cells() as f64;

                        let k_wall = (wall_mass_cell * wall_cp) / dt;
                        let d_wall = k_wall + ua_wall_cell;
                        let t_wall_new = (k_wall * pipe.wall_temp[i] + q_ext_cell + ua_wall_cell * t_fluid_new) / d_wall;
                        pipe.wall_temp[i] = t_wall_new;
                    }

                    h_in = h_new;
                }
                let h_pipe_outlet = h_in;

                // Flujo de entalpía hacia los nodos extremos
                w_out_total[u] += w;
                w_in_total[d] += w;
                phi_in[d] += w * h_pipe_outlet;
            } else {
                // ── Flujo negativo: d → u ──────────────────────────────────
                let abs_w = -w;
                let h_in_boundary = net.nodes[d].h;
                let mut h_in = h_in_boundary;

                // Space-marching de N-1 a 0 (dirección del flujo)
                for i in (0..pipe.n_cells()).rev() {
                    let rho_cell = thermo::density(pipe.cell_temp[i]);
                    let m_cell = (rho_cell * vol_cell).max(1e-10);
                    let h_cell = thermo::enthalpy(pipe.cell_temp[i]);
                    let cfl = abs_w * dt / m_cell;

                    let h_new = if pipe.wall_mass() > 0.0 {
                        let cp_cell = thermo::cp(pipe.cell_temp[i]);
                        let wall_mass_cell = pipe.wall_mass() / pipe.n_cells() as f64;
                        let wall_cp = pipe.wall_cp();
                        let ua_wall_cell = pipe.wall_ua() / pipe.n_cells() as f64;
                        let q_ext_cell = (pipe.heat_total() + pipe.q_hx_external) / pipe.n_cells() as f64;

                        let k_wall = (wall_mass_cell * wall_cp) / dt;
                        let d_wall = k_wall + ua_wall_cell;
                        let c_eff = (ua_wall_cell * k_wall) / d_wall;
                        let q_eff = (ua_wall_cell / d_wall) * q_ext_cell;

                        let ntu_w = c_eff * dt / (m_cell * cp_cell);
                        let h_wall = thermo::enthalpy(pipe.wall_temp[i]);

                        (h_cell + cfl * h_in + ntu_w * h_wall + q_eff * dt / m_cell) / (1.0 + cfl + ntu_w)
                    } else if is_hx {
                        let cp_cell = thermo::cp(pipe.cell_temp[i]);
                        let ntu_h = ua_cell * dt / (m_cell * cp_cell);
                        (h_cell + cfl * h_in + ntu_h * h_coolant) / (1.0 + cfl + ntu_h)
                    } else {
                        (h_cell + cfl * h_in + q_const_per_cell * dt / m_cell) / (1.0 + cfl)
                    };

                    let t_fluid_new = thermo::temperature_from_enthalpy(h_new);
                    _q_hx_total += ua_cell * (t_fluid_new - pipe.t_coolant());
                    pipe.cell_temp[i] = t_fluid_new;

                    if pipe.wall_mass() > 0.0 {
                        let wall_mass_cell = pipe.wall_mass() / pipe.n_cells() as f64;
                        let wall_cp = pipe.wall_cp();
                        let ua_wall_cell = pipe.wall_ua() / pipe.n_cells() as f64;
                        let q_ext_cell = (pipe.heat_total() + pipe.q_hx_external) / pipe.n_cells() as f64;

                        let k_wall = (wall_mass_cell * wall_cp) / dt;
                        let d_wall = k_wall + ua_wall_cell;
                        let t_wall_new = (k_wall * pipe.wall_temp[i] + q_ext_cell + ua_wall_cell * t_fluid_new) / d_wall;
                        pipe.wall_temp[i] = t_wall_new;
                    }

                    h_in = h_new;
                }
                let h_pipe_outlet = h_in;

                w_out_total[d] += abs_w;
                w_in_total[u] += abs_w;
                phi_in[u] += abs_w * h_pipe_outlet;
            }
        }

        // ── Actualizar entalpía/temperatura de cada nodo (esquema implícito estable) ──
        for (i, node) in net.nodes.iter_mut().enumerate() {
            if let Some(t_fixed) = node.fixed_temperature {
                node.temperature = t_fixed;
                node.h = thermo::enthalpy(t_fixed);
                continue;
            }
            let m = node.mass().max(1e-10);
            let w_in = w_in_total[i];
            let w_out = w_out_total[i];

            let h_new = if w_in >= w_out {
                let denom = m + dt * w_in;
                (m * node.h + dt * (phi_in[i] + node.external_heat)) / denom
            } else {
                let denom = m + dt * w_out;
                // Si w_out > w_in, el nodo extrae la diferencia del ambiente a su propia temperatura actual
                let env_temp = node.temperature;
                let env_heat = (w_out - w_in) * thermo::enthalpy(env_temp);
                (m * node.h + dt * (phi_in[i] + env_heat + node.external_heat)) / denom
            };
            node.h = h_new;

            // Limitar a rango físico razonable para agua líquida
            node.h = node.h.clamp(
                thermo::enthalpy(thermo::T_REF_K + 1.0),   // 1 °C (evitar congelación)
                thermo::enthalpy(thermo::T_REF_K + 110.0), // 110 °C
            );

            node.temperature = thermo::temperature_from_enthalpy(node.h);
        }
    }

    /// Busca nodos en la red que estén aislados de cualquier presión de referencia (nodos Dirichlet).
    ///
    /// Una rama se considera activa si no está completamente cerrada (p. ej. check valve cerrada
    /// o válvula de control con apertura < 1e-5).
    pub fn find_isolated_nodes(&self, net: &Network) -> Vec<usize> {
        let n_nodes = net.n_nodes();
        if n_nodes == 0 {
            return Vec::new();
        }

        let mut visited = vec![false; n_nodes];
        let mut queue = std::collections::VecDeque::new();

        // 1. Inicializar con los nodos de presión fija (Dirichlet)
        for (i, node) in net.nodes.iter().enumerate() {
            if node.is_fixed() {
                visited[i] = true;
                queue.push_back(i);
            }
        }

        // Si no hay ningún nodo Dirichlet, todos los nodos están técnicamente aislados
        if queue.is_empty() {
            return (0..n_nodes).collect();
        }

        // 2. Construir lista de adyacencia temporal de ramas activas
        let mut adj = vec![Vec::new(); n_nodes];
        for pipe in &net.pipes {
            let is_active = {
                if pipe.is_check_valve && pipe.check_valve_closed {
                    false
                } else if let crate::thnet::network::BranchComponent::Valve(_) = pipe.component {
                    pipe.valve_opening >= 1e-5
                } else {
                    true
                }
            };

            if is_active {
                adj[pipe.node_up].push(pipe.node_dn);
                adj[pipe.node_dn].push(pipe.node_up);
            }
        }

        // 3. Recorrido BFS
        while let Some(u) = queue.pop_front() {
            for &v in &adj[u] {
                if !visited[v] {
                    visited[v] = true;
                    queue.push_back(v);
                }
            }
        }

        // 4. Recopilar nodos no visitados
        let mut isolated = Vec::new();
        for i in 0..n_nodes {
            if !visited[i] {
                isolated.push(i);
            }
        }

        isolated
    }
}

impl Default for Solver {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ÁLGEBRA LINEAL AUXILIAR
// ─────────────────────────────────────────────────────────────────────────────

/// Representación de matriz dispersa por filas (LIL - List of Lists)
#[derive(Debug, Clone, Default)]
pub struct SparseMatrix {
    /// Cada elemento `rows[i]` contiene una lista de tuplas `(col_index, valor)` no nulas.
    pub rows: Vec<Vec<(usize, f64)>>,
    /// Elementos diagonales optimizados para acceso directo en Gauss-Seidel: `diag[i] = A[i][i]`
    pub diag: Vec<f64>,
}

impl SparseMatrix {
    /// Crea una matriz dispersa vacía de tamaño n x n.
    pub fn new(n: usize) -> Self {
        SparseMatrix {
            rows: vec![Vec::new(); n],
            diag: vec![0.0; n],
        }
    }

    /// Suma un valor en la celda (r, c) de la matriz.
    pub fn add(&mut self, r: usize, c: usize, val: f64) {
        if r == c {
            self.diag[r] += val;
        }
        
        if let Some(pos) = self.rows[r].iter().position(|&(col, _)| col == c) {
            self.rows[r][pos].1 += val;
        } else {
            self.rows[r].push((c, val));
        }
    }
}

/// Solvedor de sistemas lineales A·x = b mediante Gauss-Seidel con SOR.
///
/// Retorna la cantidad de iteraciones utilizadas.
pub fn gauss_seidel_sor(
    a: &SparseMatrix,
    b: &[f64],
    x: &mut [f64],
    omega: f64,
    tol: f64,
    max_iter: usize,
) -> usize {
    let n = x.len();
    if n == 0 {
        return 0;
    }

    for iter in 0..max_iter {
        let mut max_delta = 0.0_f64;
        for i in 0..n {
            let diag_val = a.diag[i];
            if diag_val.abs() < 1e-20 {
                // Evitar división por cero si la diagonal es singular
                continue;
            }

            let x_old = x[i];
            let mut sigma = b[i];
            for &(j, a_ij) in &a.rows[i] {
                if j != i {
                    sigma -= a_ij * x[j];
                }
            }
            let x_new = sigma / diag_val;
            x[i] = x_old + omega * (x_new - x_old);
            max_delta = max_delta.max((x[i] - x_old).abs());
        }

        if max_delta < tol {
            return iter + 1;
        }
    }
    max_iter
}

/// Construye el mapa node_id → índice en el sistema libre.
/// Los nodos con presión fija (Dirichlet) retornan None.
fn build_free_map(nodes: &[super::network::Node]) -> Vec<Option<usize>> {
    let mut map = Vec::with_capacity(nodes.len());
    let mut count = 0usize;
    for node in nodes {
        if node.is_fixed() {
            map.push(None);
        } else {
            map.push(Some(count));
            count += 1;
        }
    }
    map
}

/// Resolución del sistema lineal A·x = b mediante eliminación Gaussiana
/// con pivoting parcial por columna.
///
/// Válido para sistemas pequeños-medianos (≤ ~500 nodos).
/// Para redes mayores se debería usar un solver sparse (faer, nalgebra-sparse).
fn gaussian_elimination(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Vec<f64> {
    let n = b.len();
    if n == 0 {
        return Vec::new();
    }

    // Eliminación hacia adelante con pivoting parcial
    for k in 0..n {
        // Encontrar la fila con el mayor valor absoluto en la columna k
        let pivot_row = (k..n)
            .max_by(|&i, &j| {
                a[i][k]
                    .abs()
                    .partial_cmp(&a[j][k].abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(k);

        if pivot_row != k {
            a.swap(k, pivot_row);
            b.swap(k, pivot_row);
        }

        let pivot = a[k][k];
        if pivot.abs() < 1e-20 {
            // Sistema singular o cuasi-singular (red desconectada o sobredefinida)
            // Retorna la estimación actual sin continuar
            continue;
        }

        // Eliminar columna k de todas las filas inferiores
        for i in (k + 1)..n {
            let factor = a[i][k] / pivot;
            for j in k..n {
                let val = a[k][j];
                a[i][j] -= factor * val;
            }
            b[i] -= factor * b[k];
        }
    }

    // Sustitución hacia atrás
    let mut x = vec![0.0_f64; n];
    for i in (0..n).rev() {
        let mut sum = b[i];
        for j in (i + 1)..n {
            sum -= a[i][j] * x[j];
        }
        if a[i][i].abs() > 1e-20 {
            x[i] = sum / a[i][i];
        } else {
            x[i] = 0.0;
        }
    }

    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gaussian_2x2() {
        // 2x + y = 5
        // x + 3y = 10
        // Solución: x=1, y=3
        let a = vec![vec![2.0, 1.0], vec![1.0, 3.0]];
        let b = vec![5.0, 10.0];
        let x = gaussian_elimination(a, b);
        assert!((x[0] - 1.0).abs() < 1e-10, "x = {}", x[0]);
        assert!((x[1] - 3.0).abs() < 1e-10, "y = {}", x[1]);
    }

    #[test]
    fn test_check_valve() {
        use crate::thnet::network::{Network, Node, Pipe};
        
        let mut net = Network::new();
        let t_init = 20.0 + 273.15;
        
        // Nodo 0: Presión alta (2 bar)
        let id_0 = net.add_node(Node::new(t_init, 2e5, 0.001).with_fixed_pressure());
        // Nodo 1: Presión baja (1 bar)
        let id_1 = net.add_node(Node::new(t_init, 1e5, 0.001).with_fixed_pressure());
        
        // Cañería de 0 -> 1 con CheckValve
        let pipe_id = net.add_pipe(
            Pipe::new(id_0, id_1, 0.05, 10.0, 1.5e-5, 0.0, 5, t_init)
                .with_check_valve()
        );
        
        let mut solver = Solver::new();
        
        // Paso 1: Flujo a favor (de 0 a 1) -> debería fluir flujo positivo
        solver.step(&mut net, 0.1);
        assert!(net.pipes[pipe_id].flow > 0.0);
        assert!(!net.pipes[pipe_id].check_valve_closed);
        
        // Ahora invertimos las presiones (Nodo 0 = 1 bar, Nodo 1 = 2 bar)
        net.nodes[id_0].pressure = 1e5;
        net.nodes[id_0].fixed_pressure = Some(1e5);
        net.nodes[id_1].pressure = 2e5;
        net.nodes[id_1].fixed_pressure = Some(2e5);
        
        // Paso 2: El primer paso iniciará con flujo positivo (estado anterior),
        // pero la fuerza impulsora es inversa, por lo que intentará ir al revés.
        // Al final del paso, el solvedor detectará flujo negativo y cambiará el estado de la check valve
        // para el siguiente paso.
        solver.step(&mut net, 0.1);
        
        // Paso 3: Con la válvula cerrada, el caudal debe ser nulo (bloqueado por la alta resistencia)
        solver.step(&mut net, 0.1);
        assert!(net.pipes[pipe_id].check_valve_closed);
        assert!(net.pipes[pipe_id].flow.abs() < 1e-6);
    }

    #[test]
    fn test_valve_characteristics() {
        use crate::thnet::network::{Network, Node, Pipe, ValveChar};
        
        let t_init = 20.0 + 273.15;
        let mut solver = Solver::new();

        // 1. Lineal (opening = 0.5 -> f(x) = 0.5)
        let mut net_lin = Network::new();
        let id_0 = net_lin.add_node(Node::new(t_init, 2e5, 0.001).with_fixed_pressure());
        let id_1 = net_lin.add_node(Node::new(t_init, 1e5, 0.001).with_fixed_pressure());
        let p_lin = net_lin.add_pipe(
            Pipe::new(id_0, id_1, 0.05, 10.0, 1.5e-5, 0.0, 5, t_init)
                .with_valve(10.0, 0.5)
                .with_valve_char(ValveChar::Linear)
        );
        solver.step(&mut net_lin, 0.1);
        let flow_lin = net_lin.pipes[p_lin].flow;

        // 2. EqualPercentage (opening = 0.5 -> f(x) = 50^(0.5 - 1) = 50^-0.5 ≈ 0.141)
        // Como f(x)_eq = 0.141 < f(x)_lin = 0.5, el caudal debería ser menor.
        let mut net_eq = Network::new();
        let id_0 = net_eq.add_node(Node::new(t_init, 2e5, 0.001).with_fixed_pressure());
        let id_1 = net_eq.add_node(Node::new(t_init, 1e5, 0.001).with_fixed_pressure());
        let p_eq = net_eq.add_pipe(
            Pipe::new(id_0, id_1, 0.05, 10.0, 1.5e-5, 0.0, 5, t_init)
                .with_valve(10.0, 0.5)
                .with_valve_char(ValveChar::EqualPercentage)
        );
        solver.step(&mut net_eq, 0.1);
        let flow_eq = net_eq.pipes[p_eq].flow;

        // 3. QuickOpening (opening = 0.5 -> f(x) = sqrt(0.5) ≈ 0.707)
        // Como f(x)_qo = 0.707 > f(x)_lin = 0.5, el caudal debería ser mayor.
        let mut net_qo = Network::new();
        let id_0 = net_qo.add_node(Node::new(t_init, 2e5, 0.001).with_fixed_pressure());
        let id_1 = net_qo.add_node(Node::new(t_init, 1e5, 0.001).with_fixed_pressure());
        let p_qo = net_qo.add_pipe(
            Pipe::new(id_0, id_1, 0.05, 10.0, 1.5e-5, 0.0, 5, t_init)
                .with_valve(10.0, 0.5)
                .with_valve_char(ValveChar::QuickOpening)
        );
        solver.step(&mut net_qo, 0.1);
        let flow_qo = net_qo.pipes[p_qo].flow;

        // 4. Kv vs Cv (Cv = 1.156099 * Kv)
        // Si usamos with_valve_kv con 10.0 / 1.156099, debería resultar en exactamente el mismo caudal.
        let mut net_kv = Network::new();
        let id_0 = net_kv.add_node(Node::new(t_init, 2e5, 0.001).with_fixed_pressure());
        let id_1 = net_kv.add_node(Node::new(t_init, 1e5, 0.001).with_fixed_pressure());
        let p_kv = net_kv.add_pipe(
            Pipe::new(id_0, id_1, 0.05, 10.0, 1.5e-5, 0.0, 5, t_init)
                .with_valve_kv(10.0 / 1.156_099, 0.5)
                .with_valve_char(ValveChar::Linear)
        );
        solver.step(&mut net_kv, 0.1);
        let flow_kv = net_kv.pipes[p_kv].flow;

        println!("Flows @ opening=0.5: Lin={:.4}, Equal%={:.4}, QuickOpen={:.4}, Kv_converted={:.4}", flow_lin, flow_eq, flow_qo, flow_kv);
        assert!(flow_eq < flow_lin);
        assert!(flow_lin < flow_qo);
        assert!((flow_kv - flow_lin).abs() < 1e-10);
    }

    #[test]
    fn test_pump_curve_and_speed() {
        use crate::thnet::network::{Network, Node, Pipe};
        
        let t_init = 20.0 + 273.15;
        let mut solver = Solver::new();

        // 1. Bomba a velocidad nominal (s = 1.0)
        // dP_max = 3e5 Pa, w_max = 10.0 kg/s -> a2 = -3e5 / 100 = -3000 Pa·s^2/kg^2
        let mut net = Network::new();
        let id_0 = net.add_node(Node::new(t_init, 1e5, 0.001).with_fixed_pressure()); // Entrada: 1 bar
        let id_1 = net.add_node(Node::new(t_init, 2e5, 0.001).with_fixed_pressure()); // Salida: 2 bar
        let pipe_id = net.add_pipe(
            Pipe::new(id_0, id_1, 0.1, 10.0, 1.5e-5, 0.0, 5, t_init)
                .with_pump_curve(3e5, 0.0, -3000.0)
        );

        solver.step(&mut net, 0.1);
        let flow_nom = net.pipes[pipe_id].flow;
        println!("Nominal flow (s=1.0): {}", flow_nom);
        assert!(flow_nom > 0.0, "La bomba debería vencer la contrapresión y bombear hacia adelante");

        // 2. Bomba a media velocidad (s = 0.5)
        // dP_max_eff = 3e5 * 0.25 = 75000 Pa. Como 75 kPa < 100 kPa (contrapresión), el flujo debe invertirse
        net.pipes[pipe_id].pump_speed_ratio = 0.5;
        for _ in 0..15 {
            solver.step(&mut net, 0.5);
        }
        let flow_slow = net.pipes[pipe_id].flow;
        println!("Slow flow (s=0.5): {}", flow_slow);
        assert!(flow_slow < 0.0, "A baja velocidad la bomba no debería vencer la contrapresión");
    }

    #[test]
    fn test_heat_exchanger_ntu_epsilon() {
        use crate::thnet::network::{Network, Node, Pipe, HeatExchanger};

        let mut net = Network::new();
        let t_h_init = 80.0 + 273.15;
        let t_c_init = 20.0 + 273.15;

        let id_h0 = net.add_node(Node::new(t_h_init, 1e5, 0.001).with_fixed_pressure().with_fixed_temperature(t_h_init));
        let id_h1 = net.add_node(Node::new(t_h_init, 1e5, 0.001).with_fixed_pressure());

        let id_c0 = net.add_node(Node::new(t_c_init, 1e5, 0.001).with_fixed_pressure().with_fixed_temperature(t_c_init));
        let id_c1 = net.add_node(Node::new(t_c_init, 1e5, 0.001).with_fixed_pressure());

        let pipe_hot = net.add_pipe(Pipe::new(id_h0, id_h1, 0.05, 5.0, 1.5e-5, 0.0, 5, t_h_init));
        let pipe_cold = net.add_pipe(Pipe::new(id_c0, id_c1, 0.05, 5.0, 1.5e-5, 0.0, 5, t_c_init));

        net.pipes[pipe_hot].flow = 1.0;
        net.pipes[pipe_cold].flow = 1.0;

        let _hx_id = net.add_heat_exchanger(HeatExchanger::new(pipe_hot, pipe_cold, 4180.0));

        let mut solver = Solver::new();
        solver.step(&mut net, 0.1);

        let q_hot = net.pipes[pipe_hot].q_hx_external;
        let q_cold = net.pipes[pipe_cold].q_hx_external;
        println!("Q_hot: {}, Q_cold: {}", q_hot, q_cold);

        assert!(q_hot < 0.0);
        assert!(q_cold > 0.0);
        assert!((q_hot + q_cold).abs() < 1e-9);
        assert!((q_cold - 125500.0).abs() < 5000.0);
    }

    #[test]
    fn test_closed_tank_gas_and_compressibility() {
        use crate::thnet::network::{Network, Node, Pipe, ClosedTank};

        let t_init = 20.0 + 273.15;
        let mut solver = Solver::new();

        // 1. Probar tanque cerrado con colchón de gas (gas cushion)
        let mut net_gas = Network::new();
        let id_t = net_gas.add_node(Node::new(t_init, 1e5, 0.001));
        // Nodo de referencia a alta presión (3 bar) para forzar flujo hacia el tanque
        let id_ref = net_gas.add_node(Node::new(t_init, 3e5, 0.001).with_fixed_pressure());

        // Tanque cerrado de volumen total 2.0 m3, nivel inicial 1.0 m, área 0.5 m2,
        // presión de gas inicial de 1 bar (1e5 Pa), gamma = 1.4 (adiabático)
        let tank_gas = ClosedTank::new(id_t, 0.5, 1.0, 0.0, 4.0, 0.0, 2.0, 1e5, 1.4);
        net_gas.add_closed_tank(tank_gas);

        // Conectar tubería
        let p_id = net_gas.add_pipe(Pipe::new(id_ref, id_t, 0.05, 10.0, 1.5e-5, 0.0, 5, t_init));
        
        // Simular 3 pasos para ver la evolución
        for _ in 0..3 {
            solver.step(&mut net_gas, 0.5);
        }

        let tank_final = &net_gas.closed_tanks[0];
        let node_press = net_gas.nodes[id_t].pressure;
        let flow = net_gas.pipes[p_id].flow;
        println!("Closed tank gas: level={:.4} m, press={:.1} Pa, flow={:.4} kg/s", 
                 tank_final.level, node_press, flow);
        
        assert!(tank_final.level > 1.0);
        assert!(node_press > 109806.0); // Debe aumentar más allá de la presión hidrostática inicial (109.8 kPa)

        // 2. Probar tanque cerrado con compresibilidad de agua (sistema súper rígido)
        let mut net_comp = Network::new();
        let id_t2 = net_comp.add_node(Node::new(t_init, 1e5, 0.001));
        // Referencia a 20 bar (2e6 Pa)
        let id_ref2 = net_comp.add_node(Node::new(t_init, 2e6, 0.001).with_fixed_pressure());

        // Compresibilidad del agua beta = 4.5e-10 1/Pa
        let tank_comp = ClosedTank::new(id_t2, 0.5, 1.0, 0.0, 4.0, 0.0, 2.0, 1e5, 1.0)
            .with_liquid_compressibility(4.5e-10);
        net_comp.add_closed_tank(tank_comp);

        let p_id2 = net_comp.add_pipe(Pipe::new(id_ref2, id_t2, 0.05, 10.0, 1.5e-5, 0.0, 5, t_init));

        // Simular 3 pasos
        for _ in 0..3 {
            solver.step(&mut net_comp, 0.5);
        }

        let tank_final2 = &net_comp.closed_tanks[0];
        let node_press2 = net_comp.nodes[id_t2].pressure;
        let flow2 = net_comp.pipes[p_id2].flow;
        println!("Closed tank liquid: level={:.6} m, press={:.1} Pa, flow={:.6} kg/s", 
                 tank_final2.level, node_press2, flow2);

        assert!(tank_final2.level > 1.0);
        assert!(node_press2 > 1.0e6); // Sube rápidamente hacia la presión de la fuente para equilibrarse
    }

    #[test]
    fn test_stratified_tank_dynamics() {
        use crate::thnet::network::{Network, Node, Pipe, StratifiedTank};

        let t_init = 20.0 + 273.15;
        let mut solver = Solver::new();

        let mut net = Network::new();
        let id_t = net.add_node(Node::new(t_init, 1e5, 0.001));
        let id_ref = net.add_node(Node::new(t_init, 1e5, 0.001).with_fixed_pressure());

        // Tanque de 4.9 m2, nivel 19m, cota 0m, 1 bar atm, 20 capas térmicas.
        // Calefactor de 10 kW a 2m. Nozzle de entrada a 18m.
        let mut tank = StratifiedTank::new(id_t, 4.9, 19.0, 0.0, 20.0, 0.0, 1e5, 20, t_init, 2.0, 10000.0, 18.0);
        
        // Estimar caudal de salida aproximado para inicializar la entrada en balance
        let est_flow = 1990.0;
        tank.set_inlet_flow(est_flow);
        tank.set_inlet_temp(t_init);
        net.add_stratified_tank(tank);

        // Tubería corta estándar que extrae agua por gravedad/presión del tanque
        let p_id = net.add_pipe(Pipe::new(id_t, id_ref, 0.25, 5.0, 1.5e-5, 0.0, 5, t_init));
        net.pipes[p_id].flow = est_flow;
        
        // Simular 50 pasos (dt = 0.2s)
        for _ in 0..50 {
            solver.step(&mut net, 0.2);
            // Sincronizar dinámicamente el caudal de entrada con el caudal resuelto de salida
            let current_flow = net.pipes[p_id].flow;
            net.stratified_tanks[0].set_inlet_flow(current_flow);
        }

        let tank_final = &net.stratified_tanks[0];
        println!("Stratified tank: final level={:.6} m, flow={:.4} kg/s", tank_final.level(), net.pipes[p_id].flow);
        assert!((tank_final.level() - 19.0).abs() < 0.01); // Tolerancia muy estrecha ahora que está balanceado
        assert!(tank_final.layer_temp()[1] > t_init + 1e-4);
        println!("Stratified tank layer 1 temperature: {:.4} °C", tank_final.layer_temp()[1] - 273.15);
    }

    #[test]
    fn test_gauss_seidel_sor_solver() {
        use crate::thnet::network::{Network, Node, Pipe};
        
        let t_init = 20.0 + 273.15;
        
        let build_net = || {
            let mut net = Network::new();
            let id_0 = net.add_node(Node::new(t_init, 2e5, 0.001).with_fixed_pressure());
            let id_1 = net.add_node(Node::new(t_init, 1e5, 0.001).with_fixed_pressure());
            net.add_pipe(Pipe::new(id_0, id_1, 0.05, 10.0, 1.5e-5, 0.0, 5, t_init));
            net
        };

        let mut net_dense = build_net();
        let mut solver_dense = Solver::new();
        solver_dense.linear_solver = LinearSolverKind::GaussianElimination;
        solver_dense.step(&mut net_dense, 0.1);

        let mut net_sparse = build_net();
        let mut solver_sparse = Solver::new();
        solver_sparse.linear_solver = LinearSolverKind::GaussSeidelSor;
        solver_sparse.step(&mut net_sparse, 0.1);

        let flow_dense = net_dense.pipes[0].flow;
        let flow_sparse = net_sparse.pipes[0].flow;
        println!("Dense flow: {}, Sparse flow: {}", flow_dense, flow_sparse);
        assert!((flow_dense - flow_sparse).abs() < 1e-5);
    }

    #[test]
    #[should_panic(expected = "Error de topología de red: Nodos aislados detectados")]
    fn test_island_detector_panics_on_disconnection() {
        use crate::thnet::network::{Network, Node, Pipe};

        let t_init = 20.0 + 273.15;
        let mut net = Network::new();
        let id_0 = net.add_node(Node::new(t_init, 2e5, 0.001).with_fixed_pressure());
        let id_1 = net.add_node(Node::new(t_init, 1e5, 0.001));
        let _id_2 = net.add_node(Node::new(t_init, 1e5, 0.001));

        net.add_pipe(Pipe::new(id_0, id_1, 0.05, 10.0, 1.5e-5, 0.0, 5, t_init));

        let mut solver = Solver::new();
        solver.step(&mut net, 0.1);
    }

    #[test]
    fn test_find_isolated_nodes() {
        use crate::thnet::network::{Network, Node, Pipe};

        let t_init = 20.0 + 273.15;
        let mut net = Network::new();
        let id_0 = net.add_node(Node::new(t_init, 2e5, 0.001).with_fixed_pressure());
        let id_1 = net.add_node(Node::new(t_init, 1e5, 0.001));
        let id_2 = net.add_node(Node::new(t_init, 1e5, 0.001));

        net.add_pipe(Pipe::new(id_0, id_1, 0.05, 10.0, 1.5e-5, 0.0, 5, t_init));
        net.add_pipe(Pipe::new(id_1, id_2, 0.05, 10.0, 1.5e-5, 0.0, 5, t_init).with_valve(5.0, 0.0));

        let solver = Solver::new();
        let isolated = solver.find_isolated_nodes(&net);
        assert_eq!(isolated, vec![id_2]);
    }
}
