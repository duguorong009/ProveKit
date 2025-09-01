use {
    crate::witness::{digits::DigitalDecompositionWitnessesSolver, ram::SpiceWitnessesSolver},
    acir::native_types::WitnessMap,
    ark_ff::{Field, PrimeField},
    ark_std::Zero,
    provekit_common::{
        skyscraper::SkyscraperSponge,
        utils::noir_to_native,
        witness::{
            ConstantOrR1CSWitness, ConstantTerm, ProductLinearTerm, SumTerm, WitnessBuilder,
            WitnessCoefficient, BINOP_ATOMIC_BITS,
        },
        FieldElement, NoirElement,
    },
    spongefish::{codecs::arkworks_algebra::UnitToField, ProverState},
};

pub trait WitnessBuilderSolver {
    fn solve(
        &self,
        acir_witness_idx_to_value_map: &WitnessMap<NoirElement>,
        witness: &mut [Option<FieldElement>],
        transcript: &mut ProverState<SkyscraperSponge, FieldElement>,
    );
}

impl WitnessBuilderSolver for WitnessBuilder {
    fn solve(
        &self,
        acir_witness_idx_to_value_map: &WitnessMap<NoirElement>,
        witness: &mut [Option<FieldElement>],
        transcript: &mut ProverState<SkyscraperSponge, FieldElement>,
    ) {
        match self {
            WitnessBuilder::Constant(ConstantTerm(witness_idx, c)) => {
                witness[*witness_idx] = Some(*c);
            }
            WitnessBuilder::Acir(witness_idx, acir_witness_idx) => {
                witness[*witness_idx] = Some(noir_to_native(
                    *acir_witness_idx_to_value_map
                        .get_index(*acir_witness_idx as u32)
                        .unwrap(),
                ));
            }
            WitnessBuilder::Sum(witness_idx, operands) => {
                witness[*witness_idx] = Some(
                    operands
                        .iter()
                        .map(|SumTerm(coeff, witness_idx)| {
                            if let Some(coeff) = coeff {
                                *coeff * witness[*witness_idx].unwrap()
                            } else {
                                witness[*witness_idx].unwrap()
                            }
                        })
                        .fold(FieldElement::zero(), |acc, x| acc + x),
                );
            }
            WitnessBuilder::Product(witness_idx, operand_idx_a, operand_idx_b) => {
                let a: FieldElement = witness[*operand_idx_a].unwrap();
                let b: FieldElement = witness[*operand_idx_b].unwrap();
                witness[*witness_idx] = Some(a * b);
            }
            WitnessBuilder::Inverse(witness_idx, operand_idx) => {
                let operand: FieldElement = witness[*operand_idx].unwrap();
                witness[*witness_idx] = Some(operand.inverse().unwrap());
            }
            WitnessBuilder::IndexedLogUpDenominator(
                witness_idx,
                sz_challenge,
                WitnessCoefficient(index_coeff, index),
                rs_challenge,
                value,
            ) => {
                let index = witness[*index].unwrap();
                let value = witness[*value].unwrap();
                let rs_challenge = witness[*rs_challenge].unwrap();
                let sz_challenge = witness[*sz_challenge].unwrap();
                witness[*witness_idx] =
                    Some(sz_challenge - (*index_coeff * index + rs_challenge * value));
            }
            WitnessBuilder::MultiplicitiesForRange(start_idx, range_size, value_witnesses) => {
                let mut multiplicities = vec![0u32; *range_size];
                for value_witness_idx in value_witnesses {
                    // If the value is representable as just a u64, then it should be the least
                    // significant value in the BigInt representation.
                    let value = witness[*value_witness_idx].unwrap().into_bigint().0[0];
                    multiplicities[value as usize] += 1;
                }
                for (i, count) in multiplicities.iter().enumerate() {
                    witness[start_idx + i] = Some(FieldElement::from(*count));
                }
            }
            WitnessBuilder::Challenge(witness_idx) => {
                let mut one = [FieldElement::zero(); 1];
                let _ = transcript.fill_challenge_scalars(&mut one);
                witness[*witness_idx] = Some(one[0]);
            }
            WitnessBuilder::LogUpDenominator(
                witness_idx,
                sz_challenge,
                WitnessCoefficient(value_coeff, value),
            ) => {
                witness[*witness_idx] = Some(
                    witness[*sz_challenge].unwrap() - (*value_coeff * witness[*value].unwrap()),
                );
            }
            WitnessBuilder::ProductLinearOperation(
                witness_idx,
                ProductLinearTerm(x, a, b),
                ProductLinearTerm(y, c, d),
            ) => {
                witness[*witness_idx] =
                    Some((*a * witness[*x].unwrap() + *b) * (*c * witness[*y].unwrap() + *d));
            }
            WitnessBuilder::DigitalDecomposition(dd_struct) => {
                dd_struct.solve(witness);
            }
            WitnessBuilder::SpiceMultisetFactor(
                witness_idx,
                sz_challenge,
                rs_challenge,
                WitnessCoefficient(addr, addr_witness),
                value,
                WitnessCoefficient(timer, timer_witness),
            ) => {
                witness[*witness_idx] = Some(
                    witness[*sz_challenge].unwrap()
                        - (*addr * witness[*addr_witness].unwrap()
                            + witness[*rs_challenge].unwrap() * witness[*value].unwrap()
                            + witness[*rs_challenge].unwrap()
                                * witness[*rs_challenge].unwrap()
                                * *timer
                                * witness[*timer_witness].unwrap()),
                );
            }
            WitnessBuilder::SpiceWitnesses(spice_witnesses) => {
                spice_witnesses.solve(witness);
            }
            WitnessBuilder::BinOpLookupDenominator(
                witness_idx,
                sz_challenge,
                rs_challenge,
                rs_challenge_sqrd,
                lhs,
                rhs,
                output,
            ) => {
                let lhs = match lhs {
                    ConstantOrR1CSWitness::Constant(c) => *c,
                    ConstantOrR1CSWitness::Witness(witness_idx) => witness[*witness_idx].unwrap(),
                };
                let rhs = match rhs {
                    ConstantOrR1CSWitness::Constant(c) => *c,
                    ConstantOrR1CSWitness::Witness(witness_idx) => witness[*witness_idx].unwrap(),
                };
                let output = match output {
                    ConstantOrR1CSWitness::Constant(c) => *c,
                    ConstantOrR1CSWitness::Witness(witness_idx) => witness[*witness_idx].unwrap(),
                };
                witness[*witness_idx] = Some(
                    witness[*sz_challenge].unwrap()
                        - (lhs
                            + witness[*rs_challenge].unwrap() * rhs
                            + witness[*rs_challenge_sqrd].unwrap() * output),
                );
            }
            WitnessBuilder::MultiplicitiesForBinOp(witness_idx, operands) => {
                let mut multiplicities = vec![0u32; 2usize.pow(2 * BINOP_ATOMIC_BITS as u32)];
                for (lhs, rhs) in operands {
                    let lhs = match lhs {
                        ConstantOrR1CSWitness::Constant(c) => *c,
                        ConstantOrR1CSWitness::Witness(witness_idx) => {
                            witness[*witness_idx].unwrap()
                        }
                    };
                    let rhs = match rhs {
                        ConstantOrR1CSWitness::Constant(c) => *c,
                        ConstantOrR1CSWitness::Witness(witness_idx) => {
                            witness[*witness_idx].unwrap()
                        }
                    };
                    let index =
                        (lhs.into_bigint().0[0] << BINOP_ATOMIC_BITS) + rhs.into_bigint().0[0];
                    multiplicities[index as usize] += 1;
                }
                for (i, count) in multiplicities.iter().enumerate() {
                    witness[witness_idx + i] = Some(FieldElement::from(*count));
                }
            }
        }
    }
}
