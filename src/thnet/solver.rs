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

/// Solvedor acoplado hidráulico-térmico para redes de agua monofásica.
pub struct Solver {
    /// Máximo de iteraciones Newton-Raphson por paso hidráulico
    pub max_newton_iter: usize,
    /// Tolerancia de convergencia en caudal [kg/s]
    pub tol_flow: f64,
    /// Tiempo de simulación acumulado [s]
    pub time: f64,
}

impl Solver {
    pub fn new() -> Self {
        Solver {
            max_newton_iter: 50,
            tol_flow: 1e-9,
            time: 0.0,
        }
    }

    /// Avanza un paso de tiempo Δt [s].
    ///
    /// Ejecuta hidráulica (implícita) → térmica (upwind implícita).
    pub fn step(&mut self, net: &mut Network, dt: f64) {
        self.step_hydraulic(net, dt);
        self.step_thermal(net, dt);
        self.time += dt;
    }

    // ─────────────────────────────────────────────────────────────────────
    // FASE 1: HIDRÁULICA IMPLÍCITA
    // ─────────────────────────────────────────────────────────────────────

    fn step_hydraulic(&self, net: &mut Network, dt: f64) {
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
                let dp_pump = pipe.pump_dp_max;

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
            let mut c_mat = vec![vec![0.0_f64; n_free]; n_free];
            let mut b_vec = vec![0.0_f64; n_free];

            for (j, pipe) in net.pipes.iter().enumerate() {
                let u = pipe.node_up;
                let d = pipe.node_dn;
                let g = gj[j];
                let s = sj[j];

                // ── Nodo upstream u (j SALE de u → -W_j en el balance de masa) ──
                //
                // Balance en u:  ─W_j + (otras ramas) = 0
                // -W_j = -G_j·(P_u - P_d) - S_j
                //
                // Moviendo P_u al LHS y el resto al RHS:
                //   G_j·P_u  - G_j·P_d  =  -S_j
                //   ↑ LHS               ↑ RHS
                //
                // Si d es libre:   C[u,d]  -= G_j      (off-diagonal)
                // Si d es fijo:    b[u]    += G_j·P_d  (fuente conocida)
                // Fuente S_j:      b[u]    -= S_j
                if let Some(iu) = free_map[u] {
                    c_mat[iu][iu] += g;
                    b_vec[iu] -= s; // S_j va al RHS con signo negativo
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

                // ── Nodo downstream d (j LLEGA a d → +W_j en el balance de masa) ──
                //
                // Balance en d:  +W_j + (otras ramas) = 0
                // +W_j = +G_j·(P_u - P_d) + S_j
                //
                // Moviendo P_d al LHS:
                //   G_j·P_d  - G_j·P_u  =  S_j
                //   ↑ LHS               ↑ RHS
                //
                // Si u es libre:   C[d,u]  -= G_j
                // Si u es fijo:    b[d]    += G_j·P_u
                // Fuente S_j:      b[d]    += S_j
                if let Some(id) = free_map[d] {
                    c_mat[id][id] += g;
                    b_vec[id] += s; // S_j va al RHS con signo positivo
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

            // ── Resolver C·P = b (eliminación Gaussiana con pivoting parcial) ──
            let p_free = gaussian_elimination(c_mat, b_vec);

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
            let dp_pump = pipe.pump_dp_max;
            let p_u = net.nodes[pipe.node_up].pressure;
            let p_d = net.nodes[pipe.node_dn].pressure;
            let force = p_u - p_d + dp_grav + dp_pump;

            // Iteración de punto fijo con relajación para resolver la resistencia cuadrática implícita
            let mut flow_iter = pipe.flow;
            for _ in 0..20 {
                let original_flow = pipe.flow;
                pipe.flow = flow_iter;
                let r_lin = pipe.resistance_linearized();
                pipe.flow = original_flow;

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

            let a = pipe.area();
            let dz_cell = pipe.length / pipe.n_cells as f64;
            let vol_cell = a * dz_cell;

            // Modo de transferencia de calor:
            // - HX (ua_hx > 0): calor variable = UA_cell·(T_cool - T_cell), implícito
            // - Fuente constante: heat_total / n_cells, explícito en entalpía
            let is_hx = pipe.ua_hx > 0.0;
            let ua_cell = if is_hx { pipe.ua_hx / pipe.n_cells as f64 } else { 0.0 };
            let h_coolant = if is_hx {
                thermo::enthalpy(pipe.t_coolant)
            } else {
                0.0
            };
            let q_const_per_cell = if is_hx { 0.0 } else { pipe.heat_total / pipe.n_cells as f64 };

            _q_hx_total = 0.0;

            if w >= 0.0 {
                // ── Flujo positivo: u → d ──────────────────────────────────
                //
                // Esquema upwind implícito en espacio (space-marching 0→N-1).
                //
                // Ambos son incondicionalmente estables para cualquier CFL y NTU.
                let h_in_boundary = net.nodes[u].h;
                let mut h_in = h_in_boundary;

                for i in 0..pipe.n_cells {
                    let rho_cell = thermo::density(pipe.cell_temp[i]);
                    let m_cell = (rho_cell * vol_cell).max(1e-10);
                    let h_cell = thermo::enthalpy(pipe.cell_temp[i]);
                    let cfl = w * dt / m_cell;

                    let h_new = if pipe.wall_mass > 0.0 {
                        let cp_cell = thermo::cp(pipe.cell_temp[i]);
                        let wall_mass_cell = pipe.wall_mass / pipe.n_cells as f64;
                        let wall_cp = pipe.wall_cp;
                        let ua_wall_cell = pipe.wall_ua / pipe.n_cells as f64;
                        let q_ext_cell = pipe.heat_total / pipe.n_cells as f64;

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
                    _q_hx_total += ua_cell * (t_fluid_new - pipe.t_coolant);
                    pipe.cell_temp[i] = t_fluid_new;

                    if pipe.wall_mass > 0.0 {
                        let wall_mass_cell = pipe.wall_mass / pipe.n_cells as f64;
                        let wall_cp = pipe.wall_cp;
                        let ua_wall_cell = pipe.wall_ua / pipe.n_cells as f64;
                        let q_ext_cell = pipe.heat_total / pipe.n_cells as f64;

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
                for i in (0..pipe.n_cells).rev() {
                    let rho_cell = thermo::density(pipe.cell_temp[i]);
                    let m_cell = (rho_cell * vol_cell).max(1e-10);
                    let h_cell = thermo::enthalpy(pipe.cell_temp[i]);
                    let cfl = abs_w * dt / m_cell;

                    let h_new = if pipe.wall_mass > 0.0 {
                        let cp_cell = thermo::cp(pipe.cell_temp[i]);
                        let wall_mass_cell = pipe.wall_mass / pipe.n_cells as f64;
                        let wall_cp = pipe.wall_cp;
                        let ua_wall_cell = pipe.wall_ua / pipe.n_cells as f64;
                        let q_ext_cell = pipe.heat_total / pipe.n_cells as f64;

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
                    _q_hx_total += ua_cell * (t_fluid_new - pipe.t_coolant);
                    pipe.cell_temp[i] = t_fluid_new;

                    if pipe.wall_mass > 0.0 {
                        let wall_mass_cell = pipe.wall_mass / pipe.n_cells as f64;
                        let wall_cp = pipe.wall_cp;
                        let ua_wall_cell = pipe.wall_ua / pipe.n_cells as f64;
                        let q_ext_cell = pipe.heat_total / pipe.n_cells as f64;

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

}

impl Default for Solver {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ÁLGEBRA LINEAL AUXILIAR
// ─────────────────────────────────────────────────────────────────────────────

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
}
