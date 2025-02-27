pub mod algorithms;
pub mod entities;
pub mod vec;

pub use algorithms::*;
pub use entities::*;

use crate::core_crypto::gpu::vec::CudaVec;
use crate::core_crypto::prelude::{
    CiphertextModulus, DecompositionBaseLog, DecompositionLevelCount, GlweCiphertextCount,
    GlweDimension, LweBskGroupingFactor, LweCiphertextCount, LweCiphertextIndex, LweDimension,
    Numeric, PolynomialSize, UnsignedInteger,
};
use std::ffi::c_void;
use tfhe_cuda_backend::cuda_bind::*;

#[derive(Debug, Clone)]
pub struct CudaPtr {
    ptr: *mut c_void,
    device: CudaDevice,
}

#[derive(Debug, Clone)]
pub struct CudaStream {
    ptr: *mut c_void,
    device: CudaDevice,
}

impl CudaStream {
    pub fn new_unchecked(device: CudaDevice) -> Self {
        let gpu_index = device.gpu_index();
        unsafe {
            let ptr = cuda_create_stream(gpu_index);

            Self { ptr, device }
        }
    }

    /// # Safety
    ///
    /// - `stream` __must__ be a valid pointer
    pub unsafe fn as_mut_c_ptr(&mut self) -> *mut c_void {
        self.ptr
    }

    /// # Safety
    ///
    /// - `stream` __must__ be a valid pointer
    pub unsafe fn as_c_ptr(&self) -> *const c_void {
        self.ptr.cast_const()
    }

    pub fn device(&self) -> CudaDevice {
        self.device
    }

    /// Synchronizes the stream
    pub fn synchronize(&self) {
        unsafe { cuda_synchronize_stream(self.as_c_ptr()) };
    }

    /// Allocates `elements` on the GPU asynchronously
    pub fn malloc_async<T>(&self, elements: u32) -> CudaVec<T>
    where
        T: Numeric,
    {
        let size = elements as u64 * std::mem::size_of::<T>() as u64;
        unsafe {
            let ptr = CudaPtr {
                ptr: cuda_malloc_async(size, self.as_c_ptr()),
                device: self.device(),
            };

            CudaVec::new(ptr, elements as usize, self.device())
        }
    }

    pub fn memset_async<T>(&self, dest: &mut CudaVec<T>, value: T)
    where
        T: Numeric + Into<u64>,
    {
        let dest_size = dest.len() * std::mem::size_of::<T>();
        unsafe {
            cuda_memset_async(
                dest.as_mut_c_ptr(),
                value.into(),
                dest_size as u64,
                self.as_c_ptr(),
            );
        }
    }

    /// Copies data from slice into GPU pointer
    ///
    /// # Safety
    ///
    /// - `dest` __must__ be a valid pointer to the GPU global memory
    /// - [CudaDevice::cuda_synchronize_device] __must__ be called after the copy
    /// as soon as synchronization is required
    pub fn copy_to_gpu_async<T>(&self, dest: &mut CudaVec<T>, src: &[T])
    where
        T: Numeric,
    {
        let src_size = std::mem::size_of_val(src);
        assert!(dest.len() * std::mem::size_of::<T>() >= src_size);

        unsafe {
            cuda_memcpy_async_to_gpu(
                dest.as_mut_c_ptr(),
                src.as_ptr().cast(),
                src_size as u64,
                self.as_c_ptr(),
            );
        }
    }

    /// Copies data between different arrays in the GPU
    ///
    /// # Safety
    ///
    /// - `src` __must__ be a valid pointer to the GPU global memory
    /// - `dest` __must__ be a valid pointer to the GPU global memory
    /// - [CudaDevice::cuda_synchronize_device] __must__ be called after the copy
    /// as soon as synchronization is required
    pub fn copy_gpu_to_gpu_async<T>(&self, dest: &mut CudaVec<T>, src: &CudaVec<T>)
    where
        T: Numeric,
    {
        assert!(dest.len() >= src.len());
        let size = dest.len() * std::mem::size_of::<T>();

        unsafe {
            cuda_memcpy_async_gpu_to_gpu(
                dest.as_mut_c_ptr(),
                src.as_c_ptr(),
                size as u64,
                self.as_c_ptr(),
            );
        }
    }

    /// Copies data from GPU pointer into slice
    ///
    /// # Safety
    ///
    /// - `src` __must__ be a valid pointer to the GPU global memory
    /// - [CudaDevice::cuda_synchronize_device] __must__ be called as soon as synchronization is
    /// required
    pub fn copy_to_cpu_async<T>(&self, dest: &mut [T], src: &CudaVec<T>)
    where
        T: Numeric,
    {
        let dest_size = std::mem::size_of_val(dest);
        assert!(dest_size >= src.len() * std::mem::size_of::<T>());

        unsafe {
            cuda_memcpy_async_to_cpu(
                dest.as_mut_ptr().cast(),
                src.as_c_ptr(),
                dest_size as u64,
                self.as_c_ptr(),
            );
        }
    }

    /// Discarding bootstrap on a vector of LWE ciphertexts
    #[allow(clippy::too_many_arguments)]
    pub fn bootstrap_low_latency_async<T: UnsignedInteger>(
        &self,
        lwe_array_out: &mut CudaVec<T>,
        lwe_out_indexes: &CudaVec<T>,
        test_vector: &CudaVec<T>,
        test_vector_indexes: &CudaVec<T>,
        lwe_array_in: &CudaVec<T>,
        lwe_in_indexes: &CudaVec<T>,
        bootstrapping_key: &CudaVec<f64>,
        lwe_dimension: LweDimension,
        glwe_dimension: GlweDimension,
        polynomial_size: PolynomialSize,
        base_log: DecompositionBaseLog,
        level: DecompositionLevelCount,
        num_samples: u32,
        lwe_idx: LweCiphertextIndex,
    ) {
        let mut pbs_buffer: *mut i8 = std::ptr::null_mut();
        unsafe {
            scratch_cuda_bootstrap_low_latency_64(
                self.as_c_ptr(),
                std::ptr::addr_of_mut!(pbs_buffer),
                glwe_dimension.0 as u32,
                polynomial_size.0 as u32,
                level.0 as u32,
                num_samples,
                self.device().get_max_shared_memory() as u32,
                true,
            );
            cuda_bootstrap_low_latency_lwe_ciphertext_vector_64(
                self.as_c_ptr(),
                lwe_array_out.as_mut_c_ptr(),
                lwe_out_indexes.as_c_ptr(),
                test_vector.as_c_ptr(),
                test_vector_indexes.as_c_ptr(),
                lwe_array_in.as_c_ptr(),
                lwe_in_indexes.as_c_ptr(),
                bootstrapping_key.as_c_ptr(),
                pbs_buffer,
                lwe_dimension.0 as u32,
                glwe_dimension.0 as u32,
                polynomial_size.0 as u32,
                base_log.0 as u32,
                level.0 as u32,
                num_samples,
                num_samples,
                lwe_idx.0 as u32,
                self.device().get_max_shared_memory() as u32,
            );
            cleanup_cuda_bootstrap_low_latency(self.as_c_ptr(), std::ptr::addr_of_mut!(pbs_buffer));
        }
    }

    /// Discarding bootstrap on a vector of LWE ciphertexts
    #[allow(clippy::too_many_arguments)]
    pub fn bootstrap_multi_bit_async<T: UnsignedInteger>(
        &self,
        lwe_array_out: &mut CudaVec<T>,
        output_indexes: &CudaVec<T>,
        test_vector: &CudaVec<T>,
        test_vector_indexes: &CudaVec<T>,
        lwe_array_in: &CudaVec<T>,
        input_indexes: &CudaVec<T>,
        bootstrapping_key: &CudaVec<u64>,
        lwe_dimension: LweDimension,
        glwe_dimension: GlweDimension,
        polynomial_size: PolynomialSize,
        base_log: DecompositionBaseLog,
        level: DecompositionLevelCount,
        grouping_factor: LweBskGroupingFactor,
        num_samples: u32,
        lwe_idx: LweCiphertextIndex,
    ) {
        let mut pbs_buffer: *mut i8 = std::ptr::null_mut();
        unsafe {
            scratch_cuda_multi_bit_pbs_64(
                self.as_c_ptr(),
                std::ptr::addr_of_mut!(pbs_buffer),
                lwe_dimension.0 as u32,
                glwe_dimension.0 as u32,
                polynomial_size.0 as u32,
                level.0 as u32,
                grouping_factor.0 as u32,
                num_samples,
                self.device().get_max_shared_memory() as u32,
                true,
                0u32,
            );
            cuda_multi_bit_pbs_lwe_ciphertext_vector_64(
                self.as_c_ptr(),
                lwe_array_out.as_mut_c_ptr(),
                output_indexes.as_c_ptr(),
                test_vector.as_c_ptr(),
                test_vector_indexes.as_c_ptr(),
                lwe_array_in.as_c_ptr(),
                input_indexes.as_c_ptr(),
                bootstrapping_key.as_c_ptr(),
                pbs_buffer,
                lwe_dimension.0 as u32,
                glwe_dimension.0 as u32,
                polynomial_size.0 as u32,
                grouping_factor.0 as u32,
                base_log.0 as u32,
                level.0 as u32,
                num_samples,
                num_samples,
                lwe_idx.0 as u32,
                self.device().get_max_shared_memory() as u32,
                0u32,
            );
            cleanup_cuda_multi_bit_pbs(self.as_c_ptr(), std::ptr::addr_of_mut!(pbs_buffer));
        }
    }
    /// Discarding keyswitch on a vector of LWE ciphertexts
    #[allow(clippy::too_many_arguments)]
    pub fn keyswitch_async<T: UnsignedInteger>(
        &self,
        lwe_array_out: &mut CudaVec<T>,
        lwe_out_indexes: &CudaVec<T>,
        lwe_array_in: &CudaVec<T>,
        lwe_in_indexes: &CudaVec<T>,
        input_lwe_dimension: LweDimension,
        output_lwe_dimension: LweDimension,
        keyswitch_key: &CudaVec<T>,
        base_log: DecompositionBaseLog,
        l_gadget: DecompositionLevelCount,
        num_samples: u32,
    ) {
        unsafe {
            cuda_keyswitch_lwe_ciphertext_vector_64(
                self.as_c_ptr(),
                lwe_array_out.as_mut_c_ptr(),
                lwe_out_indexes.as_c_ptr(),
                lwe_array_in.as_c_ptr(),
                lwe_in_indexes.as_c_ptr(),
                keyswitch_key.as_c_ptr(),
                input_lwe_dimension.0 as u32,
                output_lwe_dimension.0 as u32,
                base_log.0 as u32,
                l_gadget.0 as u32,
                num_samples,
            );
        }
    }

    /// Convert bootstrap key
    #[allow(clippy::too_many_arguments)]
    pub fn convert_lwe_keyswitch_key_async<T: UnsignedInteger>(
        &self,
        dest: &mut CudaVec<T>,
        src: &[T],
    ) {
        self.copy_to_gpu_async(dest, src);
    }

    /// Convert bootstrap key
    #[allow(clippy::too_many_arguments)]
    pub fn convert_lwe_bootstrap_key_async<T: UnsignedInteger>(
        &self,
        dest: &mut CudaVec<f64>,
        src: &[T],
        input_lwe_dim: LweDimension,
        glwe_dim: GlweDimension,
        l_gadget: DecompositionLevelCount,
        polynomial_size: PolynomialSize,
    ) {
        let size = std::mem::size_of_val(src);
        assert_eq!(dest.len() * std::mem::size_of::<T>(), size);

        unsafe {
            cuda_convert_lwe_bootstrap_key_64(
                dest.as_mut_c_ptr(),
                src.as_ptr().cast(),
                self.as_c_ptr(),
                input_lwe_dim.0 as u32,
                glwe_dim.0 as u32,
                l_gadget.0 as u32,
                polynomial_size.0 as u32,
            );
        };
    }

    /// Convert multi-bit bootstrap key
    #[allow(clippy::too_many_arguments)]
    pub fn convert_lwe_multi_bit_bootstrap_key_async<T: UnsignedInteger>(
        &self,
        dest: &mut CudaVec<u64>,
        src: &[T],
        input_lwe_dim: LweDimension,
        glwe_dim: GlweDimension,
        l_gadget: DecompositionLevelCount,
        polynomial_size: PolynomialSize,
        grouping_factor: LweBskGroupingFactor,
    ) {
        let size = std::mem::size_of_val(src);
        assert_eq!(dest.len() * std::mem::size_of::<T>(), size);

        unsafe {
            cuda_convert_lwe_multi_bit_bootstrap_key_64(
                dest.as_mut_c_ptr(),
                src.as_ptr().cast(),
                self.as_c_ptr(),
                input_lwe_dim.0 as u32,
                glwe_dim.0 as u32,
                l_gadget.0 as u32,
                polynomial_size.0 as u32,
                grouping_factor.0 as u32,
            )
        };
    }

    /// Discarding addition of a vector of LWE ciphertexts
    pub fn add_lwe_ciphertext_vector_async<T: UnsignedInteger>(
        &self,
        lwe_array_out: &mut CudaVec<T>,
        lwe_array_in_1: &CudaVec<T>,
        lwe_array_in_2: &CudaVec<T>,
        lwe_dimension: LweDimension,
        num_samples: u32,
    ) {
        unsafe {
            cuda_add_lwe_ciphertext_vector_64(
                self.as_c_ptr(),
                lwe_array_out.as_mut_c_ptr(),
                lwe_array_in_1.as_c_ptr(),
                lwe_array_in_2.as_c_ptr(),
                lwe_dimension.0 as u32,
                num_samples,
            );
        }
    }

    /// Discarding addition of a vector of LWE ciphertexts
    pub fn add_lwe_ciphertext_vector_assign_async<T: UnsignedInteger>(
        &self,
        lwe_array_out: &mut CudaVec<T>,
        lwe_array_in: &CudaVec<T>,
        lwe_dimension: LweDimension,
        num_samples: u32,
    ) {
        unsafe {
            cuda_add_lwe_ciphertext_vector_64(
                self.as_c_ptr(),
                lwe_array_out.as_mut_c_ptr(),
                lwe_array_out.as_c_ptr(),
                lwe_array_in.as_c_ptr(),
                lwe_dimension.0 as u32,
                num_samples,
            );
        }
    }

    /// Discarding addition of a vector of LWE ciphertexts
    pub fn add_lwe_ciphertext_vector_plaintext_vector_async<T: UnsignedInteger>(
        &self,
        lwe_array_out: &mut CudaVec<T>,
        lwe_array_in: &CudaVec<T>,
        plaintext_in: &CudaVec<T>,
        lwe_dimension: LweDimension,
        num_samples: u32,
    ) {
        unsafe {
            cuda_add_lwe_ciphertext_vector_plaintext_vector_64(
                self.as_c_ptr(),
                lwe_array_out.as_mut_c_ptr(),
                lwe_array_in.as_c_ptr(),
                plaintext_in.as_c_ptr(),
                lwe_dimension.0 as u32,
                num_samples,
            );
        }
    }

    /// Discarding addition of a vector of LWE ciphertexts
    pub fn add_lwe_ciphertext_vector_plaintext_vector_assign_async<T: UnsignedInteger>(
        &self,
        lwe_array_out: &mut CudaVec<T>,
        plaintext_in: &CudaVec<T>,
        lwe_dimension: LweDimension,
        num_samples: u32,
    ) {
        unsafe {
            cuda_add_lwe_ciphertext_vector_plaintext_vector_64(
                self.as_c_ptr(),
                lwe_array_out.as_mut_c_ptr(),
                lwe_array_out.as_c_ptr(),
                plaintext_in.as_c_ptr(),
                lwe_dimension.0 as u32,
                num_samples,
            );
        }
    }

    /// Discarding negation of a vector of LWE ciphertexts
    pub fn negate_lwe_ciphertext_vector_async<T: UnsignedInteger>(
        &self,
        lwe_array_out: &mut CudaVec<T>,
        lwe_array_in: &CudaVec<T>,
        lwe_dimension: LweDimension,
        num_samples: u32,
    ) {
        unsafe {
            cuda_negate_lwe_ciphertext_vector_64(
                self.as_c_ptr(),
                lwe_array_out.as_mut_c_ptr(),
                lwe_array_in.as_c_ptr(),
                lwe_dimension.0 as u32,
                num_samples,
            );
        }
    }

    /// Discarding negation of a vector of LWE ciphertexts
    pub fn negate_lwe_ciphertext_vector_assign_async<T: UnsignedInteger>(
        &self,
        lwe_array_out: &mut CudaVec<T>,
        lwe_dimension: LweDimension,
        num_samples: u32,
    ) {
        unsafe {
            cuda_negate_lwe_ciphertext_vector_64(
                self.as_c_ptr(),
                lwe_array_out.as_mut_c_ptr(),
                lwe_array_out.as_c_ptr(),
                lwe_dimension.0 as u32,
                num_samples,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn negate_integer_radix_assign_async<T: UnsignedInteger>(
        &self,
        lwe_array: &mut CudaVec<T>,
        lwe_dimension: LweDimension,
        num_samples: u32,
        message_modulus: u32,
        carry_modulus: u32,
    ) {
        unsafe {
            cuda_negate_integer_radix_ciphertext_64_inplace(
                self.as_c_ptr(),
                lwe_array.as_mut_c_ptr(),
                lwe_dimension.0 as u32,
                num_samples,
                message_modulus,
                carry_modulus,
            );
        }
    }

    /// Discarding negation of a vector of LWE ciphertexts
    pub fn mult_lwe_ciphertext_vector_cleartext_vector_assign_async<T: UnsignedInteger>(
        &self,
        lwe_array: &mut CudaVec<T>,
        cleartext_array_in: &CudaVec<T>,
        lwe_dimension: LweDimension,
        num_samples: u32,
    ) {
        unsafe {
            cuda_mult_lwe_ciphertext_vector_cleartext_vector_64(
                self.as_c_ptr(),
                lwe_array.as_mut_c_ptr(),
                lwe_array.as_c_ptr(),
                cleartext_array_in.as_c_ptr(),
                lwe_dimension.0 as u32,
                num_samples,
            );
        }
    }

    /// Discarding negation of a vector of LWE ciphertexts
    pub fn mult_lwe_ciphertext_vector_cleartext_vector<T: UnsignedInteger>(
        &self,
        lwe_array_out: &mut CudaVec<T>,
        lwe_array_in: &CudaVec<T>,
        cleartext_array_in: &CudaVec<T>,
        lwe_dimension: LweDimension,
        num_samples: u32,
    ) {
        unsafe {
            cuda_mult_lwe_ciphertext_vector_cleartext_vector_64(
                self.as_c_ptr(),
                lwe_array_out.as_mut_c_ptr(),
                lwe_array_in.as_c_ptr(),
                cleartext_array_in.as_c_ptr(),
                lwe_dimension.0 as u32,
                num_samples,
            );
        }
    }
}

impl Drop for CudaStream {
    fn drop(&mut self) {
        self.synchronize();
        unsafe {
            cuda_destroy_stream(self.as_mut_c_ptr());
        }
    }
}

impl CudaPtr {
    /// Returns a raw pointer to the vector’s buffer.
    pub fn as_c_ptr(&self) -> *const c_void {
        self.ptr.cast_const()
    }

    /// Returns an unsafe mutable pointer to the vector’s buffer.
    pub fn as_mut_c_ptr(&mut self) -> *mut c_void {
        self.ptr
    }
}

impl Drop for CudaPtr {
    /// Free memory for pointer `ptr` synchronously
    fn drop(&mut self) {
        // Synchronizes the device to be sure no stream is still using this pointer
        let device = self.device;
        device.synchronize_device();

        // Release memory asynchronously so control returns to the CPU asap
        // let stream = CudaStream::new_unchecked(device);
        // unsafe { cuda_drop_async(self.ptr, stream.as_c_ptr(), device.gpu_index()) };
        unsafe { cuda_drop(self.as_mut_c_ptr()) };
    }
}

#[derive(Debug)]
pub struct CudaLweList<T: UnsignedInteger> {
    // Pointer to GPU data
    pub d_vec: CudaVec<T>,
    // Number of ciphertexts in the array
    pub lwe_ciphertext_count: LweCiphertextCount,
    // Lwe dimension
    pub lwe_dimension: LweDimension,
    // Ciphertext Modulus
    pub ciphertext_modulus: CiphertextModulus<T>,
}

#[derive(Debug)]
pub struct CudaGlweList<T: UnsignedInteger> {
    // Pointer to GPU data
    pub d_vec: CudaVec<T>,
    // Number of ciphertexts in the array
    pub glwe_ciphertext_count: GlweCiphertextCount,
    // Glwe dimension
    pub glwe_dimension: GlweDimension,
    // Polynomial size
    pub polynomial_size: PolynomialSize,
    // Ciphertext Modulus
    pub ciphertext_modulus: CiphertextModulus<T>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CudaDevice {
    gpu_index: u32,
}

impl CudaDevice {
    /// Creates a CudaDevice related to the GPU with index gpu_index
    pub fn new(gpu_index: u32) -> Self {
        Self { gpu_index }
    }

    pub fn gpu_index(&self) -> u32 {
        self.gpu_index
    }

    /// Synchronizes the device
    #[allow(dead_code)]
    pub fn synchronize_device(&self) {
        unsafe { cuda_synchronize_device(self.gpu_index()) };
    }

    /// Get the maximum amount of shared memory
    pub fn get_max_shared_memory(&self) -> i32 {
        unsafe { cuda_get_max_shared_memory(self.gpu_index()) }
    }

    /// Synchronizes the stream
    pub fn get_number_of_gpus(&self) -> i32 {
        unsafe { cuda_get_number_of_gpus() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_gpu_info() {
        println!("Number of GPUs: {}", unsafe { cuda_get_number_of_gpus() });
        let gpu_index: u32 = 0;
        let device = CudaDevice::new(gpu_index);
        println!("Max shared memory: {}", device.get_max_shared_memory())
    }
    #[test]
    fn allocate_and_copy() {
        let vec = vec![1_u64, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let gpu_index: u32 = 0;
        let device = CudaDevice::new(gpu_index);
        let stream = CudaStream::new_unchecked(device);
        let mut d_vec: CudaVec<u64> = stream.malloc_async::<u64>(vec.len() as u32);
        stream.copy_to_gpu_async(&mut d_vec, &vec);
        let mut empty = vec![0_u64; vec.len()];
        stream.copy_to_cpu_async(&mut empty, &d_vec);
        stream.synchronize();
        assert_eq!(vec, empty);
    }
}
