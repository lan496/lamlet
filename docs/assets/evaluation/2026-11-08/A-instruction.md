We simulate spall failure on Nb single crystals using high-velocity impact simulations using molecular dynamics (MD) with the LAMMPS code. We simulate the impact of a projectile on a target with a relative velocity of 2 km/s. The projectile is obtained by replicating the Nb BCC unit cell 20 times along the [100], [010], and [001]; the target is longer along the shock direction and is obtained by replicating the BCC unit cell 20 times along [100] and [010] and 40 times along [001]. We apply periodic boundary conditions along the directions normal to the impact direction, [001], and free boundaries along [001]. A gap of 1.5 nm initially separates the target and projectile. The system is equilibrated at 300 K for 100 ps using isothermal, isochoric MD. An impact velocity of 2 km/s is added to the thermal velocities to all the atoms in the projectile along [001] in the direction of the target. Adiabatic MD is used to simulate the impact and subsequent expansion. All atomic interactions are described using an EAM potential developed by Fellinger et al. [1] and downloaded from OpenKIM [2]. [1] Fellinger MR, Park H, Wilkins JW. Force-matched embedded-atom method potential for niobium. Physical Review B. 2010Apr;81(14):144119. doi:10.1103/PhysRevB.81.144119 [2] https://doi.org/10.25950/befb2eea.

Submit a complete LAMMPS input as `/app/submission/solution.in`. The potential file is in `/app/inputs/prompt3.potential`. The verifier runs independently on the submitted input.

Use the shell and the installed LAMMPS engine.


This is a bounded protocol-generation pilot. Submit the complete requested
schedule, but use short smoke tests during development; executing the full
100 ps is unnecessary. The initial gap is 15 Angstrom between the outermost
atomic planes. Use exactly 16000 projectile atoms and 32000 target atoms.
Use metal units, atomic atom style, axis-aligned BCC, static block regions with
explicit units, and the supplied EAM/alloy potential. The independent evaluator
supports straight-line inputs using these commands: units, dimension,
atom_style, boundary, variable, lattice, region, create_box, create_atoms, mass,
group, pair_style, pair_coeff, neighbor, neigh_modify, velocity, timestep, fix,
unfix, run, thermo, thermo_style, displace_atoms. Use static arithmetic equal
variables and nvt/nve fixes. Keep model declarations before dynamics begins.
Includes, loops, immediate expressions, dumps and other commands cannot be
assessed in this pilot. Do not add them to the final submission. Do not access
hidden verifier code or another trial's artifacts. Write only within /app.
